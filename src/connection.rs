use std::io::{Read, Write, Error, Result, ErrorKind};
use stream::Stream;
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
    pub fn new(reader: &'conn mut impl Read, writer: &'conn mut impl Write, server: bool, seed: &[u8], remote_key: &[u8]) -> Self {
        Connection {
            stream: Stream::new(server, seed, remote_key),
            reader,
            writer,
            channels: HashMap::new()
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
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut original = [0u8; 32];
        // TODO: Handle too small buffer
        let _result = self.reader.read(&mut original);

        // TODO: Error handling
        let dechunked = self.stream.dechunk(original.to_vec());
        let bytes = match dechunked.get(&0u8) {
            Some(d) => d,
            None => return Err(Error::new(ErrorKind::UnexpectedEof, "No data for default channel 0"))
        };

        if bytes.len() > buf.len() {
            return Err(Error::new(ErrorKind::WouldBlock, "Buffer is too small"))
        }

        buf.copy_from_slice(bytes.as_slice());
        Ok(bytes.len())
    }
}

impl Write for Connection<'_> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let mut result: Result<usize> = Ok(0);
        for d in self.stream.chunk(0, 0, buf.to_vec()) {
            result = match self.writer.write(d.as_slice()) {
                Ok(_) => Ok(buf.len()),
                Err(e) => return Err(Error::new(ErrorKind::Other, e.to_string())),
            };
        }
        result
    }

    fn flush(&mut self) -> Result<()> {
        self.writer.flush()
    }
}