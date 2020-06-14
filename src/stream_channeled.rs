use std::collections::HashMap;
use std::io::{Error, ErrorKind, Result};
use std::sync::mpsc::{channel as ch, Receiver, Sender};
use std::sync::Mutex;

use crate::error_str;
use crate::stream::{StreamIn, StreamOut};

pub struct StreamChanneledIn
{
    pub id: u32,
    pub stream_in: StreamIn,

    pub channels: HashMap<u8, Vec<Mutex<Sender<Vec<u8>>>>>,
}

impl StreamChanneledIn {
    pub fn read_stream(&mut self) -> Result<(u8, Vec<u8>)> {
        let packet = self.stream_in.dechunk()?;

        Ok((packet.get_channel(), packet.data))
    }

    pub fn write_channels(&self, channel: u8, data: Vec<u8>) -> Result<()> {
        let to_notify = self.channels.get(&channel).expect(format!("No Senders found for channel {}", channel).as_str());

        for sender in to_notify {
            match sender.lock() {
                Ok(d) => d.send(data.clone()).expect(format!("Unable to receive data from channel {}, receiver {:?}", channel, sender).as_str()),
                Err(_e) => return Err(error_str!("Failed to lock `send` Mutex for channel")),
            };
        }

        Ok(())
    }

    pub fn get_channel_recv(&mut self, channel: u8) -> Receiver<Vec<u8>> {
        let (sender, receiver) = ch();
        self.channels
            .entry(channel)
            .or_insert_with(Vec::new)
            .push(Mutex::new(sender));

        println!("New Sender Channel {:?}", self.channels);

        receiver
    }
}

pub struct StreamChanneledOut
{
    pub id: u32,
    pub stream_out: StreamOut,

    pub channels: HashMap<u8, Vec<Mutex<Receiver<Vec<u8>>>>>,
}

impl StreamChanneledOut {
    pub fn read_channels(&self) -> Result<HashMap<u8, Vec<Vec<u8>>>> {
        let mut result = HashMap::new();

        for channel in self.channels.keys() {
            for r in self.channels.get(channel).unwrap() {
                let data = match r.lock() {
                    Ok(d) => d.recv(),
                    Err(_e) => return Err(error_str!("Failed to lock `send` Mutex for channel")),
                };

                result
                    .entry(*channel)
                    .or_insert_with(Vec::new)
                    .push(data.expect(format!("Unable to receive data from channel {}, receiver {:?}", channel, r).as_str()));
            }
        }

        Ok(result)
    }

    pub fn write_stream(&mut self, channel: u8, data: &[u8]) -> Result<()> {
        self.stream_out.chunk(self.id, channel, data)
    }

    pub fn get_channel_send(&mut self, channel: u8) -> Sender<Vec<u8>> {
        let (sender, receiver) = ch();
        self.channels
            .entry(channel)
            .or_insert_with(Vec::new)
            .push(Mutex::new(receiver));

        println!("New Receiver Channel {:?}", self.channels);

        sender
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::BorrowMut;
    use std::fs::{File, OpenOptions};
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

        assert!(
            StreamChanneled::new(
                10,
                Box::new(file.try_clone().unwrap()),
                true,
                &[1; 32],
                &[2; 32]
            )
            .is_ok(),
            "Can't create dummy stream_channeled"
        );
        assert!(
            StreamChanneled::new(
                10,
                Box::new(file.try_clone().unwrap()),
                true,
                &[1; 31],
                &[2; 32]
            )
            .is_err(),
            "Key seed is too small, must be 32 bytes"
        );
        assert!(
            StreamChanneled::new(
                10,
                Box::new(file.try_clone().unwrap()),
                true,
                &[1; 33],
                &[2; 32]
            )
            .is_err(),
            "Key seed is too big, must be 32 bytes"
        );
        assert!(
            StreamChanneled::new(
                10,
                Box::new(file.try_clone().unwrap()),
                true,
                &[1; 32],
                &[2; 31]
            )
            .is_err(),
            "Remote key is too small, must be 32 bytes"
        );
        assert!(
            StreamChanneled::new(
                10,
                Box::new(file.try_clone().unwrap()),
                true,
                &[1; 32],
                &[2; 33]
            )
            .is_err(),
            "Remote key is too big, must be 32 bytes"
        );
        //        assert!(StreamChanneled::new(0x1FFFFFF, Box::new(file.try_clone().unwrap()), &mut rw, true, &[1; 32], &[2; 32]).is_err(), "ID is longer than 3 bytes");
        assert!(
            StreamChanneled::new(
                0xFF_FFFF,
                Box::new(file.try_clone().unwrap()),
                true,
                &[1; 32],
                &[2; 32]
            )
            .is_ok(),
            "Can't create dummy stream_channeled"
        );
        assert!(
            StreamChanneled::new(
                0xFF,
                Box::new(file.try_clone().unwrap()),
                true,
                &[1; 32],
                &[2; 32]
            )
            .is_ok(),
            "Can't create dummy stream_channeled"
        );
        assert!(
            StreamChanneled::new(
                0,
                Box::new(file.try_clone().unwrap()),
                true,
                &[1; 32],
                &[2; 32]
            )
            .is_ok(),
            "Can't create dummy stream_channeled"
        );
    }

    #[test]
    fn test_read_write() {
        let (mut conn, mut conn2) = stream_channeled_prelude();

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
    fn test_channels(mut conn: StreamChanneled, mut conn2: StreamChanneled) {
        let mut chan = conn.get_channel(4);
        let mut chan_other = conn.get_channel(0xF);

        let mut chan2 = conn2.get_channel(4);
        let mut chan2_other = conn2.get_channel(0xF);

        let thread = spawn(move || {
            test_rw(true, chan.borrow_mut(), chan2.borrow_mut(), &[7; 100]);
            test_rw(true, chan2.borrow_mut(), chan.borrow_mut(), &[7; 100]);
            test_rw(
                true,
                chan_other.borrow_mut(),
                chan2_other.borrow_mut(),
                &[7; 100],
            );
            test_rw(
                true,
                chan2_other.borrow_mut(),
                chan_other.borrow_mut(),
                &[7; 100],
            );
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
    fn stream_channeled_prelude() -> (StreamChanneled, StreamChanneled) {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(TEST_FILE_PATH)
            .unwrap();
        let conn = StreamChanneled::new(
            0xFFFF,
            Box::new(file.try_clone().unwrap()),
            false,
            &[1; 32],
            &[
                252, 59, 51, 147, 103, 165, 34, 93, 83, 169, 45, 56, 3, 35, 175, 208, 53, 215, 129,
                123, 109, 27, 228, 125, 148, 111, 107, 9, 169, 203, 220, 6,
            ],
        )
        .unwrap();

        let file2 = OpenOptions::new()
            .read(true)
            .write(true)
            .open(TEST_FILE_PATH)
            .unwrap();
        let conn2 = StreamChanneled::new(
            0xFFFF,
            Box::new(file2.try_clone().unwrap()),
            true,
            &[2; 32],
            &[
                171, 47, 202, 50, 137, 131, 34, 194, 8, 251, 45, 171, 80, 72, 189, 67, 195, 85,
                198, 67, 15, 88, 136, 151, 203, 87, 73, 97, 207, 169, 128, 111,
            ],
        )
        .unwrap();

        (conn, conn2)
    }

    #[cfg_attr(tarpaulin, skip)]
    fn test_rw(succ: bool, a: &mut impl Write, b: &mut impl Read, data: &[u8]) {
        let mut buf = [0u8; 256];

        // Should be always ok to write & flush
        match a.write(data) {
            Ok(_d) => {}
            Err(_e) => {
                return assert!(!succ, "Write should be successful");
            }
        };

        match a.flush() {
            Ok(_d) => {}
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

        if succ {
            assert_eq!(&buf[..r], data);
        } else {
            assert_ne!(&buf[..r], data);
        }
    }
}
