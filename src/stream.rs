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

#[derive(PartialEq)]
enum State {
    // TODO: Add state that the header is sent OR received before full init
    Uninitialized,
    SentHeader,
    RecvHeader,
    Done,
}

pub struct Stream {
    pub chunk_size: usize,

    pub has_id: bool,
    pub has_sequence: bool,
    pub has_data_len: bool,

    state: State,

    header: secretstream::Header,
    dec_key: secretstream::Key,

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

        let mut _pull_bytes = [0u8; secretstream::KEYBYTES];
        let mut _push_bytes = [0u8; secretstream::KEYBYTES];

        // Compute session keys
        if server {
            match kx::server_session_keys(&keys.0, &keys.1, &remote_pkey) {
                Ok((rx, tx)) => {
                    _pull_bytes = rx.0;
                    _push_bytes = tx.0;
                },
                Err(()) => return Err(StreamError::BadClientSignature)
            };
        } else {
            match kx::client_session_keys(&keys.0, &keys.1, &remote_pkey) {
                Ok((rx, tx)) => {
                    _pull_bytes = rx.0;
                    _push_bytes = tx.0;
                },
                Err(()) => return Err(StreamError::BadServerSignature)
            };
        }

        let push_key = secretstream::Key::from_slice(&_push_bytes).unwrap();
        let pull_key = secretstream::Key::from_slice(&_pull_bytes).unwrap();

        let (pusher, header) = secretstream::Stream::init_push(&push_key).unwrap();
        // This is temporary. It's wrong, we have to use the other party's header
        let puller = secretstream::Stream::init_pull(&header, &pull_key).unwrap();

        Ok(Stream {
            chunk_size: 256usize,
            has_id: false,
            has_sequence: false,
            has_data_len: false,
            enc_stream: pusher,
            dec_stream: puller,
            state: State::Uninitialized,
            header,
            dec_key: pull_key,
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

        let chunk_size = self.chunk_size - secretstream::ABYTES;
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

        if self.state != State::Done && self.state != State::SentHeader {
            chunks.push(self.header.0.to_vec());
            if self.state == State::RecvHeader { self.state = State::Done }
            else { self.state = State::SentHeader }
        }

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
                Ok(d) => {
                    if i == chunks_len(overhead) - 1 { self.enc_stream.push(d.as_slice(), None, secretstream::Tag::Push) }
                    else { self.enc_stream.push(d.as_slice(), None, secretstream::Tag::Message) }
                },
                Err(_) => return Err(StreamError::PacketSerializationError)
            };

            match cipher {
                Ok(d) => chunks.push(d),
                Err(_) => return Err(StreamError::EncryptionError)
            }

            println!("Chunked {:?}", buf);
        }

        Ok(chunks)
    }

    pub fn dechunk(&mut self, mut chunks: Vec<u8>) -> Result<HashMap<u8, Vec<u8>>, StreamError> {
        let mut data: Vec<Packet> = Vec::new();
        let mut result: HashMap<u8, Vec<u8>>  = HashMap::new();

        if self.state != State::Done && self.state != State::RecvHeader {
            let header = secretstream::Header::from_slice(&chunks[0..secretstream::HEADERBYTES]).unwrap();
            println!("{:?}", header);
            self.dec_stream = secretstream::Stream::init_pull(&header, &self.dec_key).unwrap();
            chunks = chunks[secretstream::HEADERBYTES..].to_vec();

            if self.state == State::SentHeader { self.state = State::Done }
            else { self.state = State::RecvHeader }
        }

        let chunk_size = self.chunk_size;
        let chunks_max_length = |iter| {
            if iter * chunk_size > chunks.len() { return chunks.len() }
            iter * chunk_size
        };

        for i in {0..(chunks.len() / self.chunk_size) + 1} {
            let cipher = &chunks[chunks_max_length(i)..chunks_max_length(i+1)];
            // TODO: Use the tag field? What's that None?
            let plain = match self.dec_stream.pull(cipher, None) {
                Ok(d) => d.0,
                Err(_) => return Err(StreamError::DecryptionError)
            };

            match Packet::deserialize(plain.as_slice()) {
                Ok(d) => data.push(d),
                Err(_) => return Err(StreamError::PacketDeserializationError)
            }

            println!("Dechunked {:?}", data.get(i).unwrap());
        }

        match data.get(0) {
            Some(d) => {
                if d.config.seq_len > 0 {
                    data.sort_unstable();
                }
            },
            _ => {}
        };

        for p in data {
            result.entry(p.channel).or_insert(Vec::new()).extend(p.data);
        }

        Ok(result)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::BorrowMut;

    // Known keys: vec![1; 32] -> public vec![171, 47, 202, 50, 137, 131, 34, 194, 8, 251, 45, 171, 80, 72, 189, 67, 195, 85, 198, 67, 15, 88, 136, 151, 203, 87, 73, 97, 207, 169, 128, 111]
    // Known keys: vec![2; 32] -> public vec![252, 59, 51, 147, 103, 165, 34, 93, 83, 169, 45, 56, 3, 35, 175, 208, 53, 215, 129, 123, 109, 27, 228, 125, 148, 111, 107, 9, 169, 203, 220, 6]

    #[test]
    fn new() {
        assert!(Stream::new(true, vec![1; 32].as_slice(), vec![2; 32].as_slice()).is_ok(), "A stream should be able to get created with the above config");
        assert!(Stream::new(false, vec![1; 31].as_slice(), vec![2; 32].as_slice()).is_err(), "Key seed is too small, must be 32 bytes");
        assert!(Stream::new(true, vec![1; 33].as_slice(), vec![2; 32].as_slice()).is_err(), "Key seed is too big, must be 32 bytes");
        assert!(Stream::new(false, vec![1; 32].as_slice(), vec![2; 31].as_slice()).is_err(), "Remote key is too small, must be 32 bytes");
        assert!(Stream::new(true, vec![1; 32].as_slice(), vec![2; 33].as_slice()).is_err(), "Remote key seed is too big, must be 32 bytes");
    }

    #[test]
    fn chunk_dechunk() {
        let mut server = Stream::new(true, &[2; 32], vec![171, 47, 202, 50, 137, 131, 34, 194, 8, 251, 45, 171, 80, 72, 189, 67, 195, 85, 198, 67, 15, 88, 136, 151, 203, 87, 73, 97, 207, 169, 128, 111].as_slice()).unwrap();
        let mut client = Stream::new(false, &[1; 32], vec![252, 59, 51, 147, 103, 165, 34, 93, 83, 169, 45, 56, 3, 35, 175, 208, 53, 215, 129, 123, 109, 27, 228, 125, 148, 111, 107, 9, 169, 203, 220, 6].as_slice()).unwrap();

//        test_stream_chunking(false, client.borrow_mut(), server.borrow_mut(), 0, 0, Vec::new());
//        test_stream_chunking(false, server.borrow_mut(), client.borrow_mut(), 0, 0, Vec::new());
        test_stream_chunking(true, client.borrow_mut(), server.borrow_mut(), 13, 8, vec![4u8; 512]);
        test_stream_chunking(true, server.borrow_mut(), client.borrow_mut(), 13, 8, vec![4u8; 512]);
    }

    fn test_stream_chunking(succ: bool, a: &mut Stream, b: &mut Stream, id: u32, ch: u8, data: Vec<u8>) {
        let chunked = a.chunk(id, ch, data.clone()).unwrap();
        let mut aligned: Vec<u8> = Vec::new();

        for mut chunk in chunked {
            // TODO: Find a way to take data from chunks to test each chunk
            // This does not work as the first chunk of the first test is the header
//            assert_eq!(succ, b.dechunk(chunk.clone()).is_ok());
            aligned.append(chunk.as_mut());
        }

        if succ { assert_eq!(&data, b.dechunk(aligned).unwrap().get(&ch).unwrap()); }
        else { assert_ne!(&data, b.dechunk(aligned).unwrap().get(&ch).unwrap()); }
    }
}
