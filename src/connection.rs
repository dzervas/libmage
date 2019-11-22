use std::io::{Read, Write, Error, Result, ErrorKind};
use stream::{Stream, StreamError};
use channel::Channel;
use std::sync::mpsc::{Sender, Receiver, channel as ch};
use std::collections::HashMap;

pub struct Connection<'conn> {
    stream: Stream,
    reader: &'conn mut dyn Read,
    writer: &'conn mut dyn Write,
    channels: HashMap<u8, Vec<(Sender<Vec<u8>>, Receiver<Vec<u8>>)>>
}

impl<'conn> Connection<'conn> {
    pub fn new(reader: &'conn mut impl Read, writer: &'conn mut impl Write, server: bool, seed: &[u8], remote_key: &[u8]) -> std::result::Result<Self, StreamError> {
        match Stream::new(server, seed, remote_key) {
            Ok(stream) => Ok(Connection {
                stream,
                reader,
                writer,
                channels: HashMap::new()
            }),
            Err(e) => Err(e)
        }
    }

    pub fn get_channel(&mut self, id: u32, channel: u8) -> Channel {
        let (from_ch, to_conn) = ch();
        let (from_conn, to_ch) = ch();
        self.channels.entry(channel).or_insert(Vec::new()).push((from_conn, to_conn));

        Channel {
            id,
            channel,
            sender: from_ch,
            receiver: to_ch
        }
    }

    pub fn channel_loop(&self) {
        // TODO: Implement the channel distributor
    }
}

// TODO: Add some kind of warning that this is using default id 0 and channel 0
impl Read for Connection<'_> {
    fn read(&mut self, mut buf: &mut [u8]) -> Result<usize> {
        let mut original = [0u8; 256];
        // TODO: Handle too small buffer
        let size = match self.reader.read(&mut original) {
            Ok(d) => d,
            Err(e) => return Err(e)
        };

        let dechunked = match self.stream.dechunk(original[..size].to_vec()) {
            Ok(d) => d,
            Err(_)  => return Err(Error::new(ErrorKind::InvalidData, "Error while dechunking data"))
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

impl Write for Connection<'_> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let mut result: Result<usize> = Ok(0);
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

    fn flush(&mut self) -> Result<()> {
        self.writer.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{TcpListener, TcpStream, Shutdown};
    use std::thread::{spawn, sleep};
    use std::time::Duration;

    // Known keys: vec![1; 32] -> public vec![171, 47, 202, 50, 137, 131, 34, 194, 8, 251, 45, 171, 80, 72, 189, 67, 195, 85, 198, 67, 15, 88, 136, 151, 203, 87, 73, 97, 207, 169, 128, 111]
    // Known keys: vec![2; 32] -> public vec![252, 59, 51, 147, 103, 165, 34, 93, 83, 169, 45, 56, 3, 35, 175, 208, 53, 215, 129, 123, 109, 27, 228, 125, 148, 111, 107, 9, 169, 203, 220, 6]

    #[test]
    fn read_write() {
        let listen = spawn(|| {
            let listen = TcpListener::bind("localhost:65432").unwrap();
            let sock = listen.accept().unwrap().0;

            let mut reader = sock.try_clone().unwrap();
            let mut writer = sock.try_clone().unwrap();

            let mut conn = Connection::new(&mut reader, &mut writer, true, &[2; 32], &[171, 47, 202, 50, 137, 131, 34, 194, 8, 251, 45, 171, 80, 72, 189, 67, 195, 85, 198, 67, 15, 88, 136, 151, 203, 87, 73, 97, 207, 169, 128, 111]).unwrap();

            let mut buf = [0u8; 2048];
            let mut size = 1usize;

            while size > 0 {
                size = conn.read(&mut buf).unwrap();
                conn.write(&buf[..size]).unwrap();
                conn.flush().unwrap();
            }

            sock.shutdown(Shutdown::Both);
        });

        // Shitty hack to wait for sock to bind
        sleep(Duration::from_secs(1));

        let sock = TcpStream::connect("localhost:65432").unwrap();
        let mut reader = sock.try_clone().unwrap();
        let mut writer = sock.try_clone().unwrap();

        let mut conn = Connection::new(&mut reader, &mut writer, false, &[1; 32], &[252, 59, 51, 147, 103, 165, 34, 93, 83, 169, 45, 56, 3, 35, 175, 208, 53, 215, 129, 123, 109, 27, 228, 125, 148, 111, 107, 9, 169, 203, 220, 6]).unwrap();

        let mut buf = [0u8; 2048];

        assert!(conn.write(&[7; 100]).is_ok(), "Can't write 100 bytes");
        conn.flush().unwrap();
        assert!(conn.read(&mut buf).is_ok(), "Can't read 100 bytes");
        assert_eq!(buf[0..100].to_vec(), vec![7; 100]);

        // Cleanup
        sock.shutdown(Shutdown::Both);
        listen.join().unwrap();
    }
}
