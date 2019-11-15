use std::sync::mpsc::{Sender, Receiver};
use std::io::{Read, Write, Result, Error, ErrorKind};

pub struct Channel {
    pub id: u32,
    pub channel: u8,
    pub sender: Sender<Vec<u8>>,
    pub receiver: Receiver<Vec<u8>>
}

impl Read for Channel {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let result: Result<usize> = match self.receiver.recv() {
            Ok(bytes) => {
                if bytes.len() < buf.len() {
                    return Err(Error::new(ErrorKind::WouldBlock, "Buffer is too small"))
                }
                buf.copy_from_slice(bytes.as_slice());
                Ok(bytes.len())
            },
            Err(e) => Err(Error::new(ErrorKind::Other, e.to_string())),
        };
        result
    }
}

impl Write for Channel {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let result: Result<usize> = match self.sender.send(buf.to_vec()) {
            Ok(_) => Ok(buf.len()),
            Err(e) => Err(Error::new(ErrorKind::Other, e.to_string())),
        };
        result
    }

    fn flush(&mut self) -> Result<()> {
        // Sender is unbuffered
        Ok(())
    }
}