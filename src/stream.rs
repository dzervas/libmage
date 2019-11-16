use packet::Packet;
use std::collections::HashMap;

#[derive(Clone, PartialEq, Debug)]
pub struct Stream {
    chunk_size: usize,

    pub has_id: bool,
    pub has_sequence: bool,
    pub has_data_len: bool,
}

impl Stream {
    pub fn new(chunk_size: usize, has_id: bool, has_sequence: bool, has_data_len: bool) -> Stream {
        Stream{
            chunk_size,
            has_id,
            has_sequence,
            has_data_len
        }
    }

    pub fn chunk(&self, id: u32, channel: u8, data: Vec<u8>) -> Vec<Vec<u8>> {
        let mut overhead: usize = 2;
        let mut chunks: Vec<Vec<u8>> = Vec::new();

        if self.has_id && id > 0 { overhead += (id as f64).log(0x100 as f64).ceil() as usize }
        else if self.has_id { overhead += 1 }
        if self.has_sequence { overhead += 1 }
        if self.has_data_len { overhead += 1 }

        let chunks_len = || {
            if self.chunk_size == 0 {
                return 0
            }
            if data.len() % (self.chunk_size - overhead) > 0 {
                return (data.len() / (self.chunk_size - overhead)) + 1
            }
            data.len() / (self.chunk_size - overhead)
        };

        let data_max_length = |iter| {
            if iter * (self.chunk_size - overhead) > data.len() { return data.len() }
            iter * (self.chunk_size - overhead)
        };

        for i in {0..chunks_len()} {
            let mut buf = Packet::new(
                channel,
                id,
                i as u32,
                data[data_max_length(i)..data_max_length(i+1)].to_vec()
            );

            if i == 0 { buf.is_first(true) }
            if i == chunks_len() - 1 { buf.is_last(true) }
            buf.has_id(self.has_id);
            buf.has_sequence(self.has_sequence);
            buf.has_data_len(self.has_data_len);

            // Move this to packet constructor. Add an option of max packet length
//            if self.has_seq && i == 0xff { overhead += 1 }
//            if self.has_seq && i == 0xffff { overhead += 1 }
//            if self.has_data_len && i == 0xff { overhead += 1 }
//            if self.has_data_len && i == 0xffff { overhead += 1 }
            println!("{:?}", buf);
            chunks.push(buf.serialize());
        }

        chunks
    }

    pub fn dechunk(&self, chunks: Vec<u8>) -> HashMap<u8, Vec<u8>> {
        let mut data: Vec<Packet> = Vec::new();
        let mut result: HashMap<u8, Vec<u8>>  = HashMap::new();


        let chunks_max_length = |iter| {
            if iter * self.chunk_size > chunks.len() { return chunks.len() }
            iter * self.chunk_size
        };

        // TODO: error handling
        for i in {0..(chunks.len() / self.chunk_size) + 1} {
            data.push(Packet::deserialize(&chunks[chunks_max_length(i)..chunks_max_length(i+1)]));
            println!("{:?}", data[i]);
            // TODO: config & id error handling
        }

        // TODO: error handling
        if data[0].config.seq_len > 0 {
            data.sort_unstable();
        }

        for p in data {
            result.entry(p.channel).or_insert(Vec::new()).extend(p.data);
        }

        result
    }
}
