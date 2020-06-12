use std::sync::Mutex;
use std::sync::mpsc::{Sender, Receiver};
use std::io::{Read, Write, Error, ErrorKind, Result};

use super::error_str;

pub struct Channel {
    pub sender: Mutex<Sender<Vec<u8>>>,
    pub receiver: Mutex<Receiver<Vec<u8>>>
}

impl Read for Channel {
    fn read(&mut self, mut buf: &mut [u8]) -> Result<usize> {
        let r = match self.receiver.lock() {
            Ok(d) => d,
            Err(_e) => return Err(error_str!("Failed to lock `recv` Mutex by channel"))
        };

        match r.recv() {
            Ok(bytes) => buf.write(bytes.as_slice()),
            Err(e) => Err(error_str!("Failed to recv data from channel: {}", e)),
        }
    }
}

impl Write for Channel {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let s = match self.sender.lock() {
            Ok(d) => d,
            Err(_e) => return Err(error_str!("Failed to lock `send` Mutex by channel"))
        };

        match s.send(buf.to_vec()) {
            Ok(_) => Ok(buf.len()),
            Err(e) => Err(error_str!("Failed to send data to channel: {}", e)),
        }
    }

    fn flush(&mut self) -> Result<()> {
        // Sender is unbuffered
        Ok(())
    }
}
