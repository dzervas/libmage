use packet::Packet;

use std::collections::HashMap;
use sodiumoxide::crypto::{kx, secretstream};

#[derive(Debug)]
pub enum StreamError {
    RemoteKeyError,
    SeedKeyError,
    BadClientSignature,
    BadServerSignature,
    PacketError,
    PacketSerializationError,
    PacketDeserializationError,
    EncryptionError,
    DecryptionError,
}

pub struct Stream {
    chunk_size: usize,

    pub has_id: bool,
    pub has_sequence: bool,
    pub has_data_len: bool,

    enc_stream: secretstream::Stream<secretstream::Push>,
    dec_stream: secretstream::Stream<secretstream::Pull>,
}

impl Stream {
    pub fn new(server: bool, seed: &[u8], remote_key: &[u8]) -> Result<Self, StreamError> {
        // Remote "certificate" (public key)
        let remote_pkey = match kx::PublicKey::from_slice(remote_key) {
            Some(k) => k,
            None => return Err(StreamError::RemoteKeyError)
        };

        // Actual keypair from seed
        let keys = match kx::Seed::from_slice(seed) {
            Some(k) => kx::keypair_from_seed(&k),
            None => return Err(StreamError::SeedKeyError)
        };

        let mut pull_bytes = [0u8; 32];
        let mut push_bytes = [0u8; 32];
        println!("{}: {:?}", server, keys.0);

        // Compute session keys
        if server {
            match kx::server_session_keys(&keys.0, &keys.1, &remote_pkey) {
                Ok((rx, tx)) => {
                    pull_bytes = rx.0;
                    push_bytes = tx.0;
                },
                Err(()) => return Err(StreamError::BadClientSignature)
            };
        } else {
            match kx::client_session_keys(&keys.0, &keys.1, &remote_pkey) {
                Ok((rx, tx)) => {
                    pull_bytes = rx.0;
                    push_bytes = tx.0;
                },
                Err(()) => return Err(StreamError::BadServerSignature)
            };
        }

        let pull_key = secretstream::Key::from_slice(&pull_bytes).unwrap();
        let push_key = secretstream::Key::from_slice(&push_bytes).unwrap();
        let pusher = secretstream::Stream::init_push(&push_key).unwrap();
        let puller = secretstream::Stream::init_pull(&pusher.1, &pull_key).unwrap();

        Ok(Stream {
            chunk_size: 64usize,
            has_id: false,
            has_sequence: false,
            has_data_len: false,
            enc_stream: pusher.0,
            dec_stream: puller,
        })
    }

    pub fn chunk(&mut self, id: u32, channel: u8, data: Vec<u8>) -> Result<Vec<Vec<u8>>, StreamError> {
        let mut overhead: usize = 2;
        let mut chunks: Vec<Vec<u8>> = Vec::new();

        let calc_bytes = |x| {
            if x <= 0 { return 1usize }
            (x as f64).log(0x100 as f64).ceil() as usize
        };

        if self.has_id { overhead += calc_bytes(id) }
        if self.has_sequence { overhead += 1 }
        if self.has_data_len { overhead += 1 }

        let chunk_size = self.chunk_size;
        let chunks_len = |ov| {
            if chunk_size == 0 { return 0 }
            if data.len() % (chunk_size - ov) > 0 {
                return (data.len() / (chunk_size - ov)) + 1
            }
            data.len() / (chunk_size - ov)
        };

        let data_max_length = |iter| {
            if iter * (chunk_size - overhead) > data.len() { return data.len() }
            iter * (chunk_size - overhead)
        };

        for i in {0..chunks_len(overhead)} {
            let mut buf = match Packet::new(
                channel,
                id,
                i as u32,
                data[data_max_length(i)..data_max_length(i+1)].to_vec()
            ) {
                Ok(p) => p,
                Err(_) => return Err(StreamError::PacketError)
            };

            if i == 0 { buf.first(true) }
            if i == chunks_len(overhead) - 1 { buf.last(true) }
            buf.has_id(self.has_id);
            buf.has_sequence(self.has_sequence);
            buf.has_data_len(self.has_data_len);

            // Move this to packet constructor. Add an option of max packet length
//            if self.has_sequence && i == 0xff { overhead += 1 }
//            if self.has_sequence && i == 0xffff { overhead += 1 }
//            if self.has_data_len && i == 0xff { overhead += 1 }
//            if self.has_data_len && i == 0xffff { overhead += 1 }

            // TODO: Do this internally? use it instead of overhead?
            let _ = buf.calculate_lengths();

            let cipher = match buf.serialize() {
                // TODO: Use the tag field? What's that None?
                Ok(d) => self.enc_stream.push(d.as_slice(), None, secretstream::Tag::Message),
                Err(_) => return Err(StreamError::PacketSerializationError)
            };

            match cipher {
                Ok(d) => chunks.push(d),
                Err(_) => return Err(StreamError::EncryptionError)
            }

            println!("Chunked {:?} -> {:?}", buf, chunks.get(i).unwrap());
        }

        Ok(chunks)
    }

    pub fn dechunk(&mut self, chunks: Vec<u8>) -> Result<HashMap<u8, Vec<u8>>, StreamError> {
        let mut data: Vec<Packet> = Vec::new();
        let mut result: HashMap<u8, Vec<u8>>  = HashMap::new();

        let chunk_size = self.chunk_size;
        let chunks_max_length = |iter| {
            if iter * chunk_size > chunks.len() { return chunks.len() }
            iter * chunk_size
        };

        for i in {0..(chunks.len() / self.chunk_size) + 1} {
            let d = &chunks[chunks_max_length(i)..chunks_max_length(i+1)];
            // TODO: Use the tag field? What's that None?
            let plain = match self.dec_stream.pull(d, None) {
                Ok(d) => d.0,
                Err(_) => return Err(StreamError::DecryptionError)
            };

            println!("Plain: {:?}", plain);

            match Packet::deserialize(plain.as_slice()) {
                Ok(d) => data.push(d),
                Err(_) => return Err(StreamError::PacketDeserializationError)
            }

            println!("Dechunked {:?}", data.get(i).unwrap());
        }

        match data.get(0) {
            Some(_) => data.sort_unstable(),
            _ => {}
        }

        for p in data {
            result.entry(p.channel).or_insert(Vec::new()).extend(p.data);
        }

        Ok(result)
    }
}
