use packet::Packet;

use std::collections::HashMap;
use sodiumoxide::crypto::{kx, secretstream};

pub struct Stream {
    chunk_size: usize,

    pub has_id: bool,
    pub has_sequence: bool,
    pub has_data_len: bool,

    enc_stream: secretstream::Stream<secretstream::Push>,
    dec_stream: secretstream::Stream<secretstream::Pull>,
}

impl Stream {
    pub fn new(server: bool, seed: &[u8], remote_key: &[u8]) -> Self {
        // TODO: Return Result
        // TODO: error handling
        // Remote "certificate" (public key)
        let remote_pkey = kx::PublicKey::from_slice(remote_key).unwrap();
        // Actual keypair from seed
        let keys = kx::keypair_from_seed(&kx::Seed::from_slice(seed).unwrap());
        let session_keys: (kx::SessionKey, kx::SessionKey);
        println!("{}: {:?}", server, keys.0);

        // Compute session keys
        if server {
            session_keys = match kx::server_session_keys(&keys.0, &keys.1, &remote_pkey) {
                Ok((rx, tx)) => (rx, tx),
                Err(()) => panic!("bad client signature")
            };
        } else {
            session_keys = match kx::client_session_keys(&keys.0, &keys.1, &remote_pkey) {
                Ok((rx, tx)) => (rx, tx),
                Err(()) => panic!("bad server signature")
            };
        }

        let session_bytes = ((session_keys.0).0, (session_keys.1).0);
        let stream_keys = (secretstream::Key::from_slice(&session_bytes.0).unwrap(), secretstream::Key::from_slice(&session_bytes.1).unwrap());
        let pusher = secretstream::Stream::init_push(&stream_keys.1).unwrap();
        let puller = secretstream::Stream::init_pull(&pusher.1, &stream_keys.0).unwrap();

        Stream {
            chunk_size: 64usize,
            has_id: false,
            has_sequence: false,
            has_data_len: false,
            enc_stream: pusher.0,
            dec_stream: puller,
        }
    }

    pub fn chunk(&mut self, id: u32, channel: u8, data: Vec<u8>) -> Vec<Vec<u8>> {
        let mut overhead: usize = 2;
        let mut chunks: Vec<Vec<u8>> = Vec::new();

        let calc_bytes = |x| {
            if x <= 0 { return 1usize }
            (x as f64).log(0x100 as f64).ceil() as usize
        };

        if self.has_id { overhead += calc_bytes(id) }
        if self.has_sequence { overhead += 1 }
        if self.has_data_len { overhead += 1 }

        let chunks_len = |cs| {
            if cs == 0 {
                return 0
            }
            if data.len() % (cs - overhead) > 0 {
                return (data.len() / (cs - overhead)) + 1
            }
            data.len() / (cs - overhead)
        };

        let data_max_length = |iter, cs| {
            if iter * (cs - overhead) > data.len() { return data.len() }
            iter * (cs - overhead)
        };

        for i in {0..chunks_len(self.chunk_size)} {
            let mut buf = Packet::new(
                channel,
                id,
                i as u32,
                data[data_max_length(i, self.chunk_size)..data_max_length(i+1, self.chunk_size)].to_vec()
            );

            if i == 0 { buf.is_first(true) }
            if i == chunks_len(self.chunk_size) - 1 { buf.is_last(true) }
            buf.has_id(self.has_id);
            buf.has_sequence(self.has_sequence);
            buf.has_data_len(self.has_data_len);

            // Move this to packet constructor. Add an option of max packet length
            // TODO: Implement these. Final packet is bigger than expected
//            if self.has_seq && i == 0xff { overhead += 1 }
//            if self.has_seq && i == 0xffff { overhead += 1 }
//            if self.has_data_len && i == 0xff { overhead += 1 }
//            if self.has_data_len && i == 0xffff { overhead += 1 }

            let cipher = self.enc_stream.push(buf.serialize().as_slice(), None, secretstream::Tag::Message);
            chunks.push(cipher.unwrap());
            println!("Chunked {:?} -> {:?}", buf, chunks.get(i).unwrap());
        }

        chunks
    }

    pub fn dechunk(&mut self, chunks: Vec<u8>) -> HashMap<u8, Vec<u8>> {
        let mut data: Vec<Packet> = Vec::new();
        let mut result: HashMap<u8, Vec<u8>>  = HashMap::new();


        let chunks_max_length = |iter, cs| {
            if iter * cs > chunks.len() { return chunks.len() }
            iter * cs
        };

        // TODO: error handling
        for i in {0..(chunks.len() / self.chunk_size) + 1} {
            let d = &chunks[chunks_max_length(i, self.chunk_size)..chunks_max_length(i+1, self.chunk_size)];
            let plain = self.dec_stream.pull(d, None);
            println!("Plain: {:?}", plain);
            data.push(Packet::deserialize(plain.unwrap().0.as_slice()));
            println!("Dechunked {:?}", data.get(i).unwrap());
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
