use crossbeam_channel::{Sender, Receiver};
use std::io;
use std::io::{Read, Write, Error, ErrorKind};

pub struct Channel {
    pub sender: Sender<Vec<u8>>,
    pub receiver: Receiver<Vec<u8>>
}

impl Read for Channel {
    fn read(&mut self, mut buf: &mut [u8]) -> io::Result<usize> {
        let result: io::Result<usize> = match self.receiver.recv() {
            Ok(bytes) => {
                if bytes.len() > buf.len() {
                    return Err(Error::new(ErrorKind::WouldBlock, "Buffer is too small"))
                }
                buf.write(bytes.as_slice())
            },
            Err(e) => Err(Error::new(ErrorKind::Other, e.to_string())),
        };
        result
    }
}

impl Write for Channel {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let result: io::Result<usize> = match self.sender.send(buf.to_vec()) {
            Ok(_) => Ok(buf.len()),
            Err(e) => Err(Error::new(ErrorKind::Other, e.to_string())),
        };
        result
    }

    fn flush(&mut self) -> io::Result<()> {
        // Sender is unbuffered
        Ok(())
    }
}