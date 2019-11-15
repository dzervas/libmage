use std::io::{Read, Write};
use stream::Stream;
use channel::Channel;
use std::sync::mpsc::{Sender, Receiver, channel as ch};
use std::collections::HashMap;

pub struct Connection<'conn> {
    stream: Stream,
    reader: &'conn dyn Read,
    writer: &'conn dyn Write,
    channels: HashMap<u8, Vec<(Sender<Vec<u8>>, Receiver<Vec<u8>>)>>
}

impl<'conn> Connection<'conn> {
    pub fn new(reader: &'conn impl Read, writer: &'conn impl Write, chunk_size: usize, has_id: bool, has_sequence: bool, has_data_len: bool) -> Self {
        Connection {
            stream: Stream::new(chunk_size, has_id, has_sequence, has_data_len),
            reader: reader,
            writer: writer,
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

    pub fn rw_loop(&self) {

    }
}