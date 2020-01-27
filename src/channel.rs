use std::sync::mpsc::{Sender, Receiver};
use std::io::{Read, Write, Error, ErrorKind, Result};

use super::error_str;

pub struct Channel {
    pub sender: Sender<Vec<u8>>,
    pub receiver: Receiver<Vec<u8>>
}

impl Read for Channel {
    fn read(&mut self, mut buf: &mut [u8]) -> Result<usize> {
        match self.receiver.recv() {
            Ok(bytes) => buf.write(bytes.as_slice()),
            Err(e) => Err(error_str!("Failed to recv data from channel: {}", e)),
        }
    }
}

impl Write for Channel {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        match self.sender.send(buf.to_vec()) {
            Ok(_) => Ok(buf.len()),
            Err(e) => Err(error_str!("Failed to send data to channel: {}", e)),
        }
    }

    fn flush(&mut self) -> Result<()> {
        // Sender is unbuffered
        Ok(())
    }
}
