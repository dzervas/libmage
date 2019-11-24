use std::io;
use std::io::{Read, Write, Error, ErrorKind};
use stream::{Stream, StreamError};
use channel::Channel;
use crossbeam_channel::{Sender, Receiver, bounded as ch};
use std::collections::HashMap;
use std::borrow::BorrowMut;

pub struct Connection<'conn> {
    pub id: u32,
    stream: Stream,
    reader: &'conn mut dyn Read,
    writer: &'conn mut dyn Write,
    channels: HashMap<u8, Vec<(Sender<Vec<u8>>, Receiver<Vec<u8>>)>>
}

impl<'conn> Connection<'conn> {
    pub fn new(id: u32, reader: &'conn mut impl Read, writer: &'conn mut impl Write, server: bool, seed: &[u8], remote_key: &[u8]) -> Result<Self, StreamError> {
        match Stream::new(server, seed, remote_key) {
            Ok(stream) => Ok(Connection {
                id,
                stream,
                reader,
                writer,
                channels: HashMap::new()
            }),
            Err(e) => return Err(e)
        }
    }

    pub fn read_all_channels(&mut self) -> Result<HashMap<u8, Vec<u8>>, StreamError> {
        let mut original = [0u8; 256];
        // TODO: Handle too small buffer
        // TODO: Handle errors here
        let size = match self.reader.read(&mut original) {
            Ok(d) => d,
            Err(_) => return Err(StreamError::PacketDeserializationError)
        };

        self.stream.dechunk(original[..size].to_vec())
    }

    pub fn write_channel(&mut self, channel: u8, data: Vec<u8>) -> Result<(), StreamError> {
        let packets = self.stream.chunk(0, channel, data)?;

        for p in packets {
            // TODO: Handle errors here
            self.writer.write(p.as_slice());
            // Is that needed?
            self.writer.flush();
        }

        Ok(())
    }

    pub fn get_channel(&mut self, channel: u8) -> Channel {
        let (from_ch, to_conn) = ch(0);
        let (from_conn, to_ch) = ch(0);
        self.channels.entry(channel).or_insert(Vec::new()).push((from_conn, to_conn));
        println!("{:?}", self.channels);

        Channel {
            sender: from_ch,
            receiver: to_ch,
        }
    }

    pub fn channel_loop(&mut self) {
        // TODO: Handle errors here
        for (k, v) in self.read_all_channels().unwrap().iter() {
            for c in self.channels.get(k).unwrap() {
                c.0.send(v.clone());
            }
        }

        // Maybe do this a better way?
        // Can't call write_channel inside iter cause it's already borrowed
        let mut buf: HashMap<u8, Vec<u8>> = HashMap::new();

        for (k, v) in self.channels.iter() {
            for (_, r) in v {
                match r.try_recv() {
                    Ok(d) => {
                        buf.entry(*k).or_insert(Vec::new()).append(d.clone().borrow_mut());
                    },
                    _ => {}
                };
            }
        }

        for (k, v) in buf.iter() {
            self.write_channel(*k, v.clone());
            self.flush();
        }
    }
}

// TODO: Add some kind of warning that this is using default id 0 and channel 0
impl Read for Connection<'_> {
    fn read(&mut self, mut buf: &mut [u8]) -> io::Result<usize> {
        let dechunked = self.read_all_channels().unwrap();

        let bytes = match dechunked.get(&0u8) {
            Some(d) => d,
            None => return Ok(0usize)
        };

        if bytes.len() > buf.len() {
            return Err(Error::new(ErrorKind::WouldBlock, "Buffer is too small"))
        }

        buf.write(bytes.as_slice())
    }
}

impl Write for Connection<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // TODO: Duplicate code with write_channel
        let mut result: io::Result<usize> = Ok(0);
        let chunks = match self.stream.chunk(0, 0, buf.to_vec()) {
            Ok(c) => c,
            Err(_) => return Err(Error::new(ErrorKind::InvalidData, "Error while chunking data"))
        };

        for d in chunks {
            result = match self.writer.write(d.as_slice()) {
                Ok(_) => Ok(buf.len()),
                Err(e) => return Err(e),
            };
        }

        result
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{File, OpenOptions};
    use std::io::{BufReader, BufWriter};
    use std::borrow::BorrowMut;
    use std::thread::{sleep, spawn};
    use std::time::Duration;

    // Known keys: vec![1; 32] -> public vec![171, 47, 202, 50, 137, 131, 34, 194, 8, 251, 45, 171, 80, 72, 189, 67, 195, 85, 198, 67, 15, 88, 136, 151, 203, 87, 73, 97, 207, 169, 128, 111]
    // Known keys: vec![2; 32] -> public vec![252, 59, 51, 147, 103, 165, 34, 93, 83, 169, 45, 56, 3, 35, 175, 208, 53, 215, 129, 123, 109, 27, 228, 125, 148, 111, 107, 9, 169, 203, 220, 6]

    #[test]
    fn new() {
        let file = File::create("/tmp/mage-test").unwrap();
        let mut reader = BufReader::new(file.try_clone().unwrap());
        let mut writer = BufWriter::new(file);

        assert!(Connection::new(10, &mut reader, &mut writer, true, &[1; 32], &[2; 32]).is_ok(), "Can't create dummy connection");
        assert!(Connection::new(10, &mut reader, &mut writer, true, &[1; 31], &[2; 32]).is_err(), "Key seed is too small, must be 32 bytes");
        assert!(Connection::new(10, &mut reader, &mut writer, true, &[1; 33], &[2; 32]).is_err(), "Key seed is too big, must be 32 bytes");
        assert!(Connection::new(10, &mut reader, &mut writer, true, &[1; 32], &[2; 31]).is_err(), "Remote key is too small, must be 32 bytes");
        assert!(Connection::new(10, &mut reader, &mut writer, true, &[1; 32], &[2; 33]).is_err(), "Remote key is too big, must be 32 bytes");
//        assert!(Connection::new(0x1FFFFFF, &mut reader, &mut writer, true, &[1; 32], &[2; 32]).is_err(), "ID is longer than 3 bytes");
        assert!(Connection::new(0xFFFFFF, &mut reader, &mut writer, true, &[1; 32], &[2; 32]).is_ok(), "Can't create dummy connection");
        assert!(Connection::new(0xFF, &mut reader, &mut writer, true, &[1; 32], &[2; 32]).is_ok(), "Can't create dummy connection");
        assert!(Connection::new(0, &mut reader, &mut writer, true, &[1; 32], &[2; 32]).is_ok(), "Can't create dummy connection");
    }

    #[test]
    fn read_write() {
        let file = OpenOptions::new().read(true).write(true).create(true).open("/tmp/mage-test").unwrap();
        let mut reader = BufReader::new(file.try_clone().unwrap());
        let mut writer = BufWriter::new(file);
        let mut conn = Connection::new(0xFFFF, &mut reader, &mut writer, false, &[1; 32], &[252, 59, 51, 147, 103, 165, 34, 93, 83, 169, 45, 56, 3, 35, 175, 208, 53, 215, 129, 123, 109, 27, 228, 125, 148, 111, 107, 9, 169, 203, 220, 6]).unwrap();

        let file2 = OpenOptions::new().read(true).write(true).open("/tmp/mage-test").unwrap();
        let mut reader2 = BufReader::new(file2.try_clone().unwrap());
        let mut writer2 = BufWriter::new(file2);
        let mut conn2 = Connection::new(0xFFFF, &mut reader2, &mut writer2, true, &[2; 32], &[171, 47, 202, 50, 137, 131, 34, 194, 8, 251, 45, 171, 80, 72, 189, 67, 195, 85, 198, 67, 15, 88, 136, 151, 203, 87, 73, 97, 207, 169, 128, 111]).unwrap();

        test_rw(true, conn.borrow_mut(), conn2.borrow_mut(), &[7; 100]);
        test_rw(true, conn2.borrow_mut(), conn.borrow_mut(), &[7; 100]);
        test_rw(true, conn.borrow_mut(), conn2.borrow_mut(), &[7; 1]);
        test_rw(true, conn2.borrow_mut(), conn.borrow_mut(), &[7; 1]);
//        test_rw(true, conn.borrow_mut(), conn2.borrow_mut(), &[7; 100000]);
//        test_rw(true, conn2.borrow_mut(), conn.borrow_mut(), &[7; 100000]);

        // Channels
        println!("Channels:");

        let mut chan = conn.get_channel(4);
        let mut chan_other = conn.get_channel(0xF);

        let mut chan2 = conn2.get_channel(4);
        let mut chan2_other = conn2.get_channel(0xF);

        let thread = spawn(move || {
            test_rw(true, chan.borrow_mut(), chan2.borrow_mut(), &[7; 100]);
            test_rw(true, chan2.borrow_mut(), chan.borrow_mut(), &[7; 100]);
            test_rw(true, chan_other.borrow_mut(), chan2_other.borrow_mut(), &[7; 100]);
            test_rw(true, chan2_other.borrow_mut(), chan_other.borrow_mut(), &[7; 100]);
            // This blocks
//            test_rw(false, chan.borrow_mut(), chan2_other.borrow_mut(), &[7; 100]);
//            test_rw(false, chan2_other.borrow_mut(), chan.borrow_mut(), &[7; 100]);
        });

        // I see no other way than sleep.
        // channel_loop is non-blocking (should be) and the test
        // has to end at some point
        for i in 0..6 {
            sleep(Duration::from_millis(100));
            conn.channel_loop();
            sleep(Duration::from_millis(100));
            conn2.channel_loop();
        }

        thread.join().unwrap();
    }

    #[cfg_attr(tarpaulin, skip)]
    fn test_rw(succ: bool, a: &mut impl Write, b: &mut impl Read, data: &[u8]) {
        let mut buf = [0u8; 2048];

        assert_eq!(succ, a.write(data).is_ok());
        a.flush().unwrap();
        let r = b.read(&mut buf);
        assert_eq!(succ, r.is_ok());
        if succ { assert_eq!(&buf[..r.unwrap()], data); }
    }
}
