use std::io::{Read, Write, BufRead, Result};
use std::collections::HashMap;

use bufstream::BufStream;

use crate::stream::Stream;
use crate::transport::ReadWrite;

#[cfg(feature = "channels")]
use {
    std::borrow::BorrowMut,
    std::io::{Error, ErrorKind},
    std::sync::Mutex,
    std::sync::mpsc::{Sender, Receiver, channel as ch},

    crate::error_str,
    crate::channel::Channel,
};

pub struct Connection {
    pub id: u32,
    stream: Stream,
    rw: BufStream<Box<dyn ReadWrite>>,

    #[cfg(feature = "channels")]
    channels: HashMap<u8, Vec<(Mutex<Sender<Vec<u8>>>, Mutex<Receiver<Vec<u8>>>)>>
}

impl Connection {
    pub fn new(id: u32, rw: Box<dyn ReadWrite>, server: bool, seed: &[u8], remote_key: &[u8]) -> Result<Self> {
        match Stream::new(server, seed, remote_key) {
            Ok(stream) => Ok(Connection {
                id,
                stream,
                rw: BufStream::new(rw),
                #[cfg(feature = "channels")]
                channels: HashMap::new()
            }),
            Err(e) => Err(e)
        }
    }

    pub fn read_all_channels(&mut self) -> Result<HashMap<u8, Vec<u8>>> {
        let mut result: HashMap<u8, Vec<u8>> = HashMap::new();

        // let original = self.rw.fill_buf()?;
        // let size = original.len();
        let mut original: Vec<u8> = Vec::new();
        let mut buf = [0; 1];
        while self.rw.read(&mut buf)? > 0 {
            original.push(buf[0]);
        }

        let packets = self.stream.dechunk(original.as_slice())?;

        // self.rw.consume(size);

        for p in packets {
            result.entry(p.get_channel()).or_insert_with(Vec::new).extend(p.data);
        }

        Ok(result)
    }

    pub fn write_channel(&mut self, channel: u8, data: &[u8]) -> Result<usize> {
        let packets = self.stream.chunk(0, channel, data)?;
        let mut result: usize = 0;

        for p in packets {
            result += self.rw.write(p.as_slice())?;
        }

        // Is this needed?
        self.rw.flush()?;

        Ok(result)
    }

    #[cfg(feature = "channels")]
    pub fn get_channel(&mut self, channel: u8) -> Channel {
        let (from_ch, to_conn) = ch();
        let (from_conn, to_ch) = ch();
        self.channels.entry(channel).or_insert_with(Vec::new).push((Mutex::new(from_conn), Mutex::new(to_conn)));
        println!("New Channel {:?}", self.channels);

        Channel {
            sender: Mutex::new(from_ch),
            receiver: Mutex::new(to_ch),
        }
    }

    #[cfg(feature = "channels")]
    pub fn channel_loop(&mut self) -> Result<()> {
        println!("\tStart send");
        self.channel_loop_send()?;
        println!("\tStart recv");
        self.channel_loop_recv()?;
        println!("\tEnd!");
        Ok(())
    }

    #[cfg(feature = "channels")]
    pub fn channel_loop_send(&mut self) -> Result<()> {
        for (chan, data) in self.read_all_channels()?.iter() {
            println!("Looping");
            let channels = match self.channels.get(chan) {
                Some(d) => d,
                None => continue
            };

            for c in channels {
                let r = match c.0.lock() {
                    Ok(d) => d.send(data.clone()),
                    Err(_e) => return Err(error_str!("Failed to lock `send` Mutex for channel"))
                };
                if r.is_err() { return Err(error_str!("Unable to send message")); }
            }
        }
        println!("Read all channels");

        Ok(())
    }

    #[cfg(feature = "channels")]
    pub fn channel_loop_recv(&mut self) -> Result<()> {
        // The data is first buffered to the HashMap buf and then sent
        // Can't call write_channel inside iter cause it's already borrowed
        // Have I mentioned before that I want the borrow checked to go fuck
        // itself? No? Happy to do so :)
        let mut buf: HashMap<u8, Vec<u8>> = HashMap::new();

        for (chan, receivers) in self.channels.iter() {
            for (_send, recv) in receivers {
                let r = match recv.lock() {
                    Ok(d) => d,
                    Err(_e) => return Err(error_str!("Failed to lock `send` Mutex for channel"))
                };

                if let Ok(d) = r.try_recv() {
                    buf.entry(*chan)
                        .or_insert_with(Vec::new)
                        .append(d.to_vec().borrow_mut())
                }
            }
        }
        println!("Read all data to channels");

        for (chan, data) in buf.iter() {
            self.write_channel(*chan, data.as_slice())?;
        }
        println!("Sent all data to channels");

        Ok(())
    }
}

#[deprecated(since="0.1.0", note="Please use `read_all_channels` or `channel_loop` with `get_channel`")]
impl Read for Connection {
    fn read(&mut self, mut buf: &mut [u8]) -> Result<usize> {
        let dechunked = self.read_all_channels()?;

        // Dunno how to hit "None" in a test
        let bytes = match dechunked.get(&0u8) {
            Some(d) => d.as_slice(),
            None => &[]
        };

        buf.write(bytes)
    }
}

#[deprecated(since="0.1.0", note="Please use `write_channel` or `channel_loop` with `get_channel`")]
impl Write for Connection {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.write_channel(0, buf)
    }

    fn flush(&mut self) -> Result<()> {
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
        assert!(Connection::new(0xFF_FFFF, Box::new(file.try_clone().unwrap()), true, &[1; 32], &[2; 32]).is_ok(), "Can't create dummy connection");
        assert!(Connection::new(0xFF, Box::new(file.try_clone().unwrap()), true, &[1; 32], &[2; 32]).is_ok(), "Can't create dummy connection");
        assert!(Connection::new(0, Box::new(file.try_clone().unwrap()), true, &[1; 32], &[2; 32]).is_ok(), "Can't create dummy connection");
    }

    #[test]
    fn test_read_write() {
        let (mut conn, mut conn2) = connection_prelude();

        test_rw(true, conn.borrow_mut(), conn2.borrow_mut(), &[7; 100]);
        test_rw(true, conn2.borrow_mut(), conn.borrow_mut(), &[7; 100]);
        test_rw(true, conn.borrow_mut(), conn2.borrow_mut(), &[]);
        test_rw(true, conn2.borrow_mut(), conn.borrow_mut(), &[]);
        conn.write_channel(1, &[1; 10]).unwrap();
        let mut buf = [5; 10];
        assert_eq!(conn2.read(&mut buf).unwrap(), 0);
        // These pollute the buffers!
//        test_rw(true, conn.borrow_mut(), conn2.borrow_mut(), &[7; 100000]);
//        test_rw(true, conn2.borrow_mut(), conn.borrow_mut(), &[7; 100000]);

        #[cfg(feature = "channels")]
        test_channels(conn, conn2);
    }

//    #[test]
    #[cfg(feature = "channels")]
    fn test_channels(mut conn: Connection, mut conn2: Connection) {
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
        for _ in 0..8 {
            sleep(Duration::from_millis(100));
            conn.channel_loop().unwrap();
            sleep(Duration::from_millis(100));
            conn2.channel_loop().unwrap();
        }

        thread.join().unwrap();
    }

    #[cfg_attr(tarpaulin, skip)]
    fn connection_prelude() -> (Connection, Connection) {
        let file = OpenOptions::new().read(true).write(true).create(true).open(TEST_FILE_PATH).unwrap();
        let conn = Connection::new(0xFFFF, Box::new(file.try_clone().unwrap()), false, &[1; 32], &[252, 59, 51, 147, 103, 165, 34, 93, 83, 169, 45, 56, 3, 35, 175, 208, 53, 215, 129, 123, 109, 27, 228, 125, 148, 111, 107, 9, 169, 203, 220, 6]).unwrap();

        let file2 = OpenOptions::new().read(true).write(true).open(TEST_FILE_PATH).unwrap();
        let conn2 = Connection::new(0xFFFF, Box::new(file2.try_clone().unwrap()), true, &[2; 32], &[171, 47, 202, 50, 137, 131, 34, 194, 8, 251, 45, 171, 80, 72, 189, 67, 195, 85, 198, 67, 15, 88, 136, 151, 203, 87, 73, 97, 207, 169, 128, 111]).unwrap();

        (conn, conn2)
    }

    #[cfg_attr(tarpaulin, skip)]
    fn test_rw(succ: bool, a: &mut impl Write, b: &mut impl Read, data: &[u8]) {
        let mut buf = [0u8; 256];

        // Should be always ok to write & flush
        match a.write(data) {
            Ok(_d) => {},
            Err(_e) => {
                return assert!(!succ, "Write should be successful");
            }
        };

        match a.flush() {
            Ok(_d) => {},
            Err(_e) => {
                return assert!(!succ, "Write should be successful");
            }
        };

        let r = match b.read(&mut buf) {
            Ok(d) => d,
            Err(_e) => {
                return assert!(!succ, "Write should be successful");
            }
        };

        if succ { assert_eq!(&buf[..r], data); }
        else { assert_ne!(&buf[..r], data); }
    }
}
