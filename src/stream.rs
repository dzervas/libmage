use std::io::{Error, ErrorKind, Result};

use super::error_str;
use crate::packet::{Packet, PacketConfig};

use sodiumoxide::crypto::{kx, secretstream};

#[derive(PartialEq, Debug)]
enum State {
    Uninitialized,
    SentHeader,
    RecvHeader,
    Done,
}

pub struct Stream {
    packet_config: PacketConfig,

    state: State,

    header: secretstream::Header,
    dec_key: secretstream::Key,

    enc_stream: secretstream::Stream<secretstream::Push>,
    dec_stream: secretstream::Stream<secretstream::Pull>,
}

impl Stream {
    pub fn new(server: bool, seed: &[u8], remote_key: &[u8]) -> Result<Self> {
        // Remote "certificate" (public key) - can't recover from this...
        let remote_pkey = match kx::PublicKey::from_slice(remote_key) {
            Some(d) => d,
            None => return Err(error_str!("Unable to remote key. Is the key 32 bytes?"))
        };

        // Actual keypair from seed - can't recover from this...
        let keys = kx::keypair_from_seed(&match kx::Seed::from_slice(seed) {
            Some(d) => d,
            None => return Err(error_str!("Unable to initialize keys from seed. Is the seed 32 bytes?"))
        });

        let mut _pull_bytes = [0u8; secretstream::KEYBYTES];
        let mut _push_bytes = [0u8; secretstream::KEYBYTES];

        // Compute session keys
        let (rx, tx) = if server {
            kx::server_session_keys(&keys.0, &keys.1, &remote_pkey).unwrap()
        } else {
            kx::client_session_keys(&keys.0, &keys.1, &remote_pkey).unwrap()
        };

        _pull_bytes = rx.0;
        _push_bytes = tx.0;

        let push_key = secretstream::Key::from_slice(&_push_bytes).unwrap();
        let pull_key = secretstream::Key::from_slice(&_pull_bytes).unwrap();

        let (pusher, header) = secretstream::Stream::init_push(&push_key).unwrap();
        // This is temporary. It's wrong, we have to use the other party's header
        let puller = secretstream::Stream::init_pull(&header, &pull_key).unwrap();

        Ok(Stream {
            // Tarpaulin has a bug and wants the whole struct in 1 line
            packet_config: PacketConfig { has_id: true, has_sequence: true, has_data_len: true, max_size: 256 - secretstream::ABYTES, },
            enc_stream: pusher,
            dec_stream: puller,
            state: State::Uninitialized,
            header,
            dec_key: pull_key,
        })
    }

    pub fn chunk(&mut self, id: u32, channel: u8, data: &[u8]) -> Result<Vec<Vec<u8>>> {
        let mut chunks: Vec<Vec<u8>> = Vec::new();

        if self.state != State::Done && self.state != State::SentHeader {
            // Send the header to remote to init their decryption stream
            chunks.push(self.header.0.to_vec());
            if self.state == State::RecvHeader { self.state = State::Done }
            else { self.state = State::SentHeader }
        }

        let mut i: u32 = 0;
        let mut written: usize = 0;

        while {
            let (plain, w) = match self.packet_config.serialize(id, channel, i, &data[written..]) {
                Ok(d) => d,
                Err(e) => return Err(e)
            };

            written += w;
            i += 1;

            // I can't find any case where encrypt fails
            let cipher = self.enc_stream.push(&plain.as_slice(), None, secretstream::Tag::Message).unwrap();
            #[cfg(not(test))]
            println!("Chunked {}: {:?}", cipher.len(), &cipher);
            chunks.push(cipher);

            // NOTE: This is do..while, check https://gist.github.com/huonw/8435502
            written < data.len()
        } {}

        Ok(chunks)
    }

    pub fn dechunk(&mut self, mut chunks: &[u8]) -> Result<Vec<Packet>> {
        let mut result: Vec<Packet> = Vec::new();

        if self.state != State::Done && self.state != State::RecvHeader {
            // Parse the header and init the decryption stream
            if chunks.len() < secretstream::HEADERBYTES {
                return Err(error_str!("Could not initialize the decryption stream (received header too small?)"))
            }

            let header = secretstream::Header::from_slice(&chunks[..secretstream::HEADERBYTES]).unwrap();
            // I can't find any case where decryption init fails
            self.dec_stream = secretstream::Stream::init_pull(&header, &self.dec_key).unwrap();

            chunks = &chunks[secretstream::HEADERBYTES..];

            if self.state == State::SentHeader { self.state = State::Done }
            else { self.state = State::RecvHeader }
        }

        // 2 is the minimum mage header
        if chunks.len() < 2 { return Ok(Vec::new()) }

        let mut read: usize = 0;

        while {
            let max_size = if chunks.len() > self.packet_config.max_size + read + secretstream::ABYTES {
                read + self.packet_config.max_size + secretstream::ABYTES
            } else { chunks.len() };
            #[cfg(not(test))]
            println!("Dechunking {}: {:?}", chunks[read..max_size].len(), &chunks[read..max_size]);

            // Do something with the tag?
            let (plain, _tag) = match self.dec_stream.pull(&chunks[read..max_size], None) {
                Ok(d) => d,
                Err(_) => return Err(error_str!("Could not decrypt packet"))
            };

            let (pack, _pc, r) = PacketConfig::deserialize(plain.as_slice());
            // While I think it's a good idea to error out on different configs, max_size can't be
            // calculated if we don't have data_len enabled, as a smaller packet will have smaller
            // max_size (due to less data). Maybe I should implement that logic in the Eq trait
//            if pc != self.packet_config { return Err(Box::new(StreamError::PacketConfigDiffersError)) }

            read += r + secretstream::ABYTES;
            result.push(pack);

            // NOTE: This is do..while, check https://gist.github.com/huonw/8435502
            read < chunks.len()
        } {}

        if self.packet_config.has_sequence { result.sort() }

        Ok(result)
    }

    // Settings
    #[allow(dead_code)]
    pub fn id(&mut self, v: bool) -> &mut Self { self.packet_config.has_id = v; self }
    #[allow(dead_code)]
    pub fn sequence(&mut self, v: bool) -> &mut Self { self.packet_config.has_sequence = v; self }
    #[allow(dead_code)]
    pub fn data_len(&mut self, v: bool) -> &mut Self { self.packet_config.has_data_len = v; self }
    #[allow(dead_code)]
    pub fn max_size(&mut self, v: usize) -> &mut Self { self.packet_config.max_size = v; self }
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
        // TODO: Test out-of-order and lost chunks
        let mut server = Stream::new(true, &[2; 32], vec![171, 47, 202, 50, 137, 131, 34, 194, 8, 251, 45, 171, 80, 72, 189, 67, 195, 85, 198, 67, 15, 88, 136, 151, 203, 87, 73, 97, 207, 169, 128, 111].as_slice()).unwrap();
        let mut client = Stream::new(false, &[1; 32], vec![252, 59, 51, 147, 103, 165, 34, 93, 83, 169, 45, 56, 3, 35, 175, 208, 53, 215, 129, 123, 109, 27, 228, 125, 148, 111, 107, 9, 169, 203, 220, 6].as_slice()).unwrap();

        let chunked = client.chunk(0, 0, &[1; 2]).unwrap();
        let mut data: Vec<u8> = Vec::new();

        for chunk in chunked {
            data.extend(chunk);
        }

        assert!(server.dechunk(&data[..3]).is_err(), "Server shouldn't be able to even initialize the ephemeral key");
        assert!(server.dechunk(&data[..]).is_ok(), "Server should be able to dechunk this and initialize the ephemeral key");
        assert!(server.dechunk(&data[..3]).is_err(), "Server shouldn't be able to dechunk partial messages");

        client.max_size(100);
        server.max_size(100);

        test_stream_chunking(true, client.borrow_mut(), server.borrow_mut(), 0, 0, &[]);
        test_stream_chunking(true, server.borrow_mut(), client.borrow_mut(), 0, 0, &[]);
        client.id(false);
        test_stream_chunking(true, client.borrow_mut(), server.borrow_mut(), 13, 8, &[3u8; 4]);
        test_stream_chunking(true, server.borrow_mut(), client.borrow_mut(), 13, 8, &[3u8; 4]);
        client.data_len(false);
        test_stream_chunking(false, server.borrow_mut(), client.borrow_mut(), 0x1FF_FFFF, 8, &[4u8; 512]);
        test_stream_chunking(false, server.borrow_mut(), client.borrow_mut(), 0x1_FFFF, 0x1F, &[4u8; 512]);
        client.sequence(false);
        test_stream_chunking(true, client.borrow_mut(), server.borrow_mut(), 13, 8, &[4u8; 512]);
        test_stream_chunking(true, server.borrow_mut(), client.borrow_mut(), 13, 8, &[4u8; 512]);
    }

    #[cfg_attr(tarpaulin, skip)]
    fn test_stream_chunking(succ: bool, a: &mut Stream, b: &mut Stream, id: u32, ch: u8, data: &[u8]) {
        let chunked = match a.chunk(id, ch, &data) {
            Ok(c) => c,
            Err(_e) => {
                return assert!(succ, "Chunk should have been created!");
            }
        };
        let mut aligned: Vec<u8> = Vec::new();
        let mut findat: Vec<u8> = Vec::new();

        for chunk in chunked {
            aligned.extend(chunk);
        }

        let dechunked = match b.dechunk(aligned.as_slice()) {
            Ok(d) => d,
            Err(_e) => {
                return assert!(succ, "Chunk should have been created!");
            }
        };

        for d in dechunked {
            findat.extend(d.data);
        }

        if succ { assert_eq!(&data, &findat.as_slice()); }
        else { assert_ne!(&data, &findat.as_slice()); }
    }
}
