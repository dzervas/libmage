use packet;

#[derive(Clone, PartialEq, Debug)]
pub struct Stream {
    chunk_size: usize,

    pub id: u32,
    pub has_id: bool,
    pub has_seq: bool,
    pub has_data_len: bool,

//    channels: Vec<>
}

impl Stream {
    pub fn new(id: u32, chunk_size: usize) -> Stream {
        Stream{
            chunk_size,
            id,
            has_id: true,
            has_seq: false,
            has_data_len: false
        }
    }

    pub fn chunk(&self, channel: u8, data: Vec<u8>) -> Vec<packet::Packet> {
        let mut overhead: usize = 2;
        let mut chunks: Vec<packet::Packet> = Vec::new();

        if self.has_id { overhead += (self.id as f64).log(0x100 as f64).ceil() as usize }
        if self.has_seq { overhead += 1 }
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
            chunks.push(packet::Packet::new(
                channel,
                self.id,
                i as u32,
                data[data_max_length(i)..data_max_length(i+1)].to_vec()
            ));

            if i == 0 { chunks[i].is_first(true) }
            if i == chunks_len() - 1 { chunks[i].is_last(true) }

            // Move this to packet constructor. Add an option of max packet length
//            if self.has_seq && i == 0xff { overhead += 1 }
//            if self.has_seq && i == 0xffff { overhead += 1 }
//            if self.has_data_len && i == 0xff { overhead += 1 }
//            if self.has_data_len && i == 0xffff { overhead += 1 }
        }

        chunks
    }

//    pub fn dechunk(data: &[u8]) -> Vec<packet::Packet> {
//        let packets: Vec<packet::Packet> = vec![];
//
//    }
}
