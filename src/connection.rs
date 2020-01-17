use std::io;
use std::io::{Read, Write, Error, ErrorKind};
use stream::Stream;
use channel::Channel;
use std::sync::mpsc::{Sender, Receiver, channel as ch};
use std::collections::HashMap;
use std::borrow::BorrowMut;
use transport::ReadWrite;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub struct Connection {
    pub id: u32,
    stream: Stream,
    rw: Box<dyn ReadWrite>,
    channels: HashMap<u8, Vec<(Sender<Vec<u8>>, Receiver<Vec<u8>>)>>
}

impl Connection {
    pub fn new(id: u32, rw: Box<dyn ReadWrite>, server: bool, seed: &[u8], remote_key: &[u8]) -> Result<Self> {
        match Stream::new(server, seed, remote_key) {
            Ok(stream) => Ok(Connection {
                id,
                stream,
                rw,
                channels: HashMap::new()
            }),
            Err(e) => Err(e)
        }
    }

    pub fn read_all_channels(&mut self) -> Result<HashMap<u8, Vec<u8>>> {
        let mut result: HashMap<u8, Vec<u8>> = HashMap::new();
        let mut original = vec![];
        let size = self.rw.read_to_end(&mut original)?;

        let packets = self.stream.dechunk(&original[..size])?;

        for p in packets {
            result.entry(p.get_channel()).or_insert(Vec::new()).extend(p.data);
        }

        Ok(result)
    }

    pub fn write_channel(&mut self, channel: u8, data: &[u8]) -> Result<usize> {
        let packets = self.stream.chunk(0, channel, data)?;
        let mut result: usize = 0;

        for p in packets {
            result += self.rw.write(p.as_slice())?;
            // Is that needed?
            self.rw.flush()?;
        }

        Ok(result)
    }

    #[allow(dead_code)]
    fn get_channel(&mut self, channel: u8) -> Channel {
        let (from_ch, to_conn) = ch();
        let (from_conn, to_ch) = ch();
        self.channels.entry(channel).or_insert(Vec::new()).push((from_conn, to_conn));
        println!("{:?}", self.channels);

        Channel {
            sender: from_ch,
            receiver: to_ch,
        }
    }

    #[allow(dead_code)]
    fn channel_loop(&mut self) -> Result<()> {
        for (k, v) in self.read_all_channels().unwrap().iter() {
            for c in self.channels.get(k).unwrap() {
                c.0.send(v.clone())?;
            }
        }

        // Maybe do this a better way?
        // Can't call write_channel inside iter cause it's already borrowed
        let mut buf: HashMap<u8, Vec<u8>> = HashMap::new();

        for (k, v) in self.channels.iter() {
            for (_, r) in v {
                let d = r.try_recv();
                if d.is_ok() {
                    buf.entry(*k).or_insert(Vec::new()).append(d.unwrap().to_vec().borrow_mut());
                }
            }
        }

        for (k, v) in buf.iter() {
            self.write_channel(*k, v.as_slice())?;
            self.flush()?;
        }

        Ok(())
    }
}

#[deprecated(since="0.1.0", note="Please use `read_all_channels` or `channel_loop` with `get_channel`")]
impl Read for Connection {
    fn read(&mut self, mut buf: &mut [u8]) -> io::Result<usize> {
        let dechunked = match self.read_all_channels() {
            Ok(d) => d,
            Err(e) => return Err(io::Error::new(ErrorKind::Other, e.to_string()))
        };

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

#[deprecated(since="0.1.0", note="Please use `write_channel` or `channel_loop` with `get_channel`")]
impl Write for Connection {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self.write_channel(0, buf) {
            Ok(d) => Ok(d),
            Err(e) => Err(io::Error::new(ErrorKind::Other, e.to_string()))
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.rw.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{File, OpenOptions};
    use std::borrow::BorrowMut;
    use std::thread::{sleep, spawn};
    use std::time::Duration;

    // Known keys: vec![1; 32] -> public vec![171, 47, 202, 50, 137, 131, 34, 194, 8, 251, 45, 171, 80, 72, 189, 67, 195, 85, 198, 67, 15, 88, 136, 151, 203, 87, 73, 97, 207, 169, 128, 111]
    // Known keys: vec![2; 32] -> public vec![252, 59, 51, 147, 103, 165, 34, 93, 83, 169, 45, 56, 3, 35, 175, 208, 53, 215, 129, 123, 109, 27, 228, 125, 148, 111, 107, 9, 169, 203, 220, 6]

    #[cfg(target_os = "windows")]
    const TEST_FILE_PATH: &str = ".mage-test.tmp";

    #[cfg(not(target_os = "windows"))]
    const TEST_FILE_PATH: &str = ".mage-test.tmp";

    #[test]
    fn new() {
        let file = File::create(TEST_FILE_PATH).unwrap();

        assert!(Connection::new(10, Box::new(file.try_clone().unwrap()), true, &[1; 32], &[2; 32]).is_ok(), "Can't create dummy connection");
        assert!(Connection::new(10, Box::new(file.try_clone().unwrap()), true, &[1; 31], &[2; 32]).is_err(), "Key seed is too small, must be 32 bytes");
        assert!(Connection::new(10, Box::new(file.try_clone().unwrap()), true, &[1; 33], &[2; 32]).is_err(), "Key seed is too big, must be 32 bytes");
        assert!(Connection::new(10, Box::new(file.try_clone().unwrap()), true, &[1; 32], &[2; 31]).is_err(), "Remote key is too small, must be 32 bytes");
        assert!(Connection::new(10, Box::new(file.try_clone().unwrap()), true, &[1; 32], &[2; 33]).is_err(), "Remote key is too big, must be 32 bytes");
//        assert!(Connection::new(0x1FFFFFF, Box::new(file.try_clone().unwrap()), &mut rw, true, &[1; 32], &[2; 32]).is_err(), "ID is longer than 3 bytes");
        assert!(Connection::new(0xFFFFFF, Box::new(file.try_clone().unwrap()), true, &[1; 32], &[2; 32]).is_ok(), "Can't create dummy connection");
        assert!(Connection::new(0xFF, Box::new(file.try_clone().unwrap()), true, &[1; 32], &[2; 32]).is_ok(), "Can't create dummy connection");
        assert!(Connection::new(0, Box::new(file.try_clone().unwrap()), true, &[1; 32], &[2; 32]).is_ok(), "Can't create dummy connection");
    }

    #[test]
    fn read_write() {
        let file = OpenOptions::new().read(true).write(true).create(true).open(TEST_FILE_PATH).unwrap();
        let mut conn = Connection::new(0xFFFF, Box::new(file.try_clone().unwrap()), false, &[1; 32], &[252, 59, 51, 147, 103, 165, 34, 93, 83, 169, 45, 56, 3, 35, 175, 208, 53, 215, 129, 123, 109, 27, 228, 125, 148, 111, 107, 9, 169, 203, 220, 6]).unwrap();

        let file2 = OpenOptions::new().read(true).write(true).open(TEST_FILE_PATH).unwrap();
        let mut conn2 = Connection::new(0xFFFF, Box::new(file2.try_clone().unwrap()), true, &[2; 32], &[171, 47, 202, 50, 137, 131, 34, 194, 8, 251, 45, 171, 80, 72, 189, 67, 195, 85, 198, 67, 15, 88, 136, 151, 203, 87, 73, 97, 207, 169, 128, 111]).unwrap();

        test_rw(true, conn.borrow_mut(), conn2.borrow_mut(), &[7; 100]);
        test_rw(true, conn2.borrow_mut(), conn.borrow_mut(), &[7; 100]);
        test_rw(true, conn.borrow_mut(), conn2.borrow_mut(), &[]);
        test_rw(true, conn2.borrow_mut(), conn.borrow_mut(), &[]);
        test_rw(false, conn.borrow_mut(), conn2.borrow_mut(), &[7; 100000]);
        test_rw(false, conn2.borrow_mut(), conn.borrow_mut(), &[7; 100000]);

        // Channels
        let mut chan = conn.get_channel(4);
        let mut chan_other = conn.get_channel(0xF);

        let mut chan2 = conn2.get_channel(4);
        let mut chan2_other = conn2.get_channel(0xF);

        let thread = spawn(move || {
            test_rw(true, chan.borrow_mut(), chan2.borrow_mut(), &[7; 100]);
            test_rw(true, chan2.borrow_mut(), chan.borrow_mut(), &[7; 100]);
            test_rw(true, chan_other.borrow_mut(), chan2_other.borrow_mut(), &[7; 100]);
            test_rw(true, chan2_other.borrow_mut(), chan_other.borrow_mut(), &[7; 100]);
            // TODO: Find a way to test blocking channels
//            test_rw(false, chan_other.borrow_mut(), chan2_other.borrow_mut(), &[7; 100000]);
//            test_rw(false, chan2_other.borrow_mut(), chan_other.borrow_mut(), &[7; 100000]);
//            test_rw(false, chan.borrow_mut(), chan2_other.borrow_mut(), &[7; 100]);
//            test_rw(false, chan2_other.borrow_mut(), chan.borrow_mut(), &[7; 100]);
        });

        // I see no other way than sleep.
        // channel_loop is non-blocking (should be) and the test
        // has to end at some point
        for _ in 0..6 {
            sleep(Duration::from_millis(100));
            conn.channel_loop().unwrap();
            sleep(Duration::from_millis(100));
            conn2.channel_loop().unwrap();
        }

        thread.join().unwrap();
    }

    #[cfg_attr(tarpaulin, skip)]
    fn test_rw(succ: bool, a: &mut impl Write, b: &mut impl Read, data: &[u8]) {
        let mut buf = [0u8; 256];

        // Should be always ok to write & flush
        match a.write(data) {
            Ok(_d) => {},
            Err(_e) => {
                if succ { assert!(true, "Write should be successful"); }
                else { return; }
            }
        };

        match a.flush() {
            Ok(_d) => {},
            Err(_e) => {
                if succ { assert!(true, "Write should be successful"); }
                else { return; }
            }
        };

        let r = match b.read(&mut buf) {
            Ok(d) => d,
            Err(_e) => {
                if succ { return assert!(true, "Write should be successful"); }
                else { return; }
            }
        };

        if succ { assert_eq!(&buf[..r], data); }
        else { assert_ne!(&buf[..r], data); }
    }
}
