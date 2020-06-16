use std::io::{Error, ErrorKind, Read, Result, Write};

use super::error_str;
use crate::packet::{Packet, PacketConfig};

use sodiumoxide::crypto::{kx, secretstream};

pub fn exchange_keys(
    mut reader: Box<dyn Read + Send + Sync>,
    mut writer: Box<dyn Write + Send + Sync>,
    is_server: bool,
    private_key_seed: &[u8],
    remote_public_key: &[u8],
) -> Result<(StreamIn, StreamOut)>
{
    // Remote "certificate" (public key) - can't recover from this...
    let remote_public_key_kx = match kx::PublicKey::from_slice(remote_public_key) {
        Some(d) => d,
        None => {
            return Err(error_str!(
                "Unable to remote public key. Is the key 32 bytes?"
            ))
        }
    };

    // Actual keypair from seed - can't recover from this...
    let keys = kx::keypair_from_seed(&match kx::Seed::from_slice(private_key_seed) {
        Some(d) => d,
        None => {
            return Err(error_str!(
                "Unable to initialize keys from private seed. Is the seed 32 bytes?"
            ))
        }
    });

    // Compute session keys
    // These are the ephemeral keys generated after the Blake key exchange
    // One has to be a "server" and one the "client" - which has nothing to do
    // with actual network topology, it's only naming.
    // Two parties have to have the opposite value in order to communicate
    let (rx, tx) = if is_server {
        kx::server_session_keys(&keys.0, &keys.1, &remote_public_key_kx).unwrap()
    } else {
        kx::client_session_keys(&keys.0, &keys.1, &remote_public_key_kx).unwrap()
    };

    let pull_bytes = rx.0;
    let push_bytes = tx.0;

    // One stream is created to send data (push) and one to receive (pull)
    let push_key = secretstream::Key::from_slice(&push_bytes).unwrap();
    let pull_key = secretstream::Key::from_slice(&pull_bytes).unwrap();

    let (pusher, header) = secretstream::Stream::init_push(&push_key).unwrap();
    let mut remote_header_bytes: [u8; secretstream::HEADERBYTES] = [0; secretstream::HEADERBYTES];

    if is_server {
        reader.read_exact(&mut remote_header_bytes)?;
        writer.write_all(&header.0)?;
    } else {
        writer.write_all(&header.0)?;
        reader.read_exact(&mut remote_header_bytes)?;
    }

    let remote_header = secretstream::Header::from_slice(&remote_header_bytes)
        .expect("Unable to decode remote header");
    let puller = secretstream::Stream::init_pull(&remote_header, &pull_key).unwrap();

    Ok((
        StreamIn {
            // TODO: Make these per-transport configuratble
            packet_config: PacketConfig {
                has_id: true,
                has_sequence: true,
                has_data_len: true,
                max_size: 256 - secretstream::ABYTES,
            },
            reader: Box::new(reader) as Box<dyn Read + Send + Sync>,
            puller,
        },
        StreamOut {
            packet_config: PacketConfig {
                has_id: true,
                has_sequence: true,
                has_data_len: true,
                max_size: 256 - secretstream::ABYTES,
            },
            writer: Box::new(writer) as Box<dyn Write + Send + Sync>,
            pusher,
        },
    ))
}

pub struct StreamIn
{
    packet_config: PacketConfig,
    reader: Box<dyn Read + Send + Sync>,
    puller: secretstream::Stream<secretstream::Pull>,
}

impl StreamIn
{
    pub fn dechunk(&mut self) -> Result<Packet> {
        let mut read_bytes: Vec<u8> = Vec::new();

        let mut byte = [0];
        let mut plaintext: Vec<u8> = Vec::new();
        let mut _tag: secretstream::Tag = secretstream::Tag::Message;

        #[allow(irrefutable_let_patterns)]
        while let byte_result = self.reader.read(&mut byte) {
            if let Err(e) = byte_result {
                match e.kind() {
                    ErrorKind::Interrupted | ErrorKind::UnexpectedEof | ErrorKind::WouldBlock => {}
                    _ => return Err(e),
                }
            } else if read_bytes.len() >= secretstream::ABYTES + self.packet_config.max_size {
                break;
            } else {
                read_bytes.push(byte[0]);
            }

            if read_bytes.len() <= secretstream::ABYTES {
                continue;
            }

            // Do something with the tag?
            if let Ok(d) = self.puller.pull(read_bytes.as_slice(), None) {
                plaintext = d.0;
                _tag = d.1;
                break;
            }
        }

        if plaintext.is_empty() {
            return Err(error_str!("Could not decrypt packet"));
        }

        #[cfg(not(test))]
        println!("Dechunked {}: {:?}", read_bytes.len(), read_bytes);

        let (packet, _config, _deserialized_bytes) = PacketConfig::deserialize(plaintext.as_slice());
        // While I think it's a good idea to error out on different configs, max_size can't be
        // calculated if we don't have data_len enabled, as a smaller packet will have smaller
        // max_size (due to less data). Maybe I should implement that logic in the Eq trait

        Ok(packet)
    }
}

pub struct StreamOut
{
    packet_config: PacketConfig,
    writer: Box<dyn Write + Send + Sync>,
    pusher: secretstream::Stream<secretstream::Push>,
}

impl StreamOut
{
    pub fn chunk(&mut self, id: u32, channel: u8, data: &[u8]) -> Result<()> {
        let mut i: u32 = 0;
        let mut chunked: usize = 0;

        while {
            let (plain, w) = match self
                .packet_config
                .serialize(id, channel, i, &data[chunked..])
            {
                Ok(d) => d,
                Err(e) => return Err(e),
            };

            chunked += w;
            i += 1;

            // I can't find any case where encrypt fails
            let cipher = self
                .pusher
                .push(&plain.as_slice(), None, secretstream::Tag::Message)
                .unwrap();
            #[cfg(not(test))]
            println!("Chunked {}: {:?}", cipher.len(), &cipher);
            self.writer.write_all(&mut cipher.as_slice())?;

            // NOTE: This is do..while, check https://gist.github.com/huonw/8435502
            chunked < data.len()
        } {}

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::BorrowMut;
    use std::fs::OpenOptions;

    // Known keys: vec![1; 32] -> public vec![171, 47, 202, 50, 137, 131, 34, 194, 8, 251, 45, 171, 80, 72, 189, 67, 195, 85, 198, 67, 15, 88, 136, 151, 203, 87, 73, 97, 207, 169, 128, 111]
    // Known keys: vec![2; 32] -> public vec![252, 59, 51, 147, 103, 165, 34, 93, 83, 169, 45, 56, 3, 35, 175, 208, 53, 215, 129, 123, 109, 27, 228, 125, 148, 111, 107, 9, 169, 203, 220, 6]

    #[cfg(target_os = "windows")]
    const TEST_FILE_PATH: &str = ".mage-test.tmp";

    #[cfg(not(target_os = "windows"))]
    const TEST_FILE_PATH: &str = ".mage-test.tmp";

    #[cfg_attr(tarpaulin, skip)]
    fn stream_prelude() -> ((StreamIn, StreamOut), (StreamIn, StreamOut)) {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(TEST_FILE_PATH)
            .unwrap();
        let file_clone = file.try_clone().unwrap();

        let conn = exchange_keys(Box::new(file), Box::new(file_clone), false, &[1; 32],
            &[
                252, 59, 51, 147, 103, 165, 34, 93, 83, 169, 45, 56, 3, 35, 175, 208, 53, 215, 129,
                123, 109, 27, 228, 125, 148, 111, 107, 9, 169, 203, 220, 6,
            ],
    ).unwrap();

        let file2 = OpenOptions::new()
            .read(true)
            .write(true)
            .open(TEST_FILE_PATH)
            .unwrap();
        let file2_clone = file2.try_clone().unwrap();

        let conn2 = exchange_keys(Box::new(file2), Box::new(file2_clone), true, &[2; 32],
            &[
                171, 47, 202, 50, 137, 131, 34, 194, 8, 251, 45, 171, 80, 72, 189, 67, 195, 85,
                198, 67, 15, 88, 136, 151, 203, 87, 73, 97, 207, 169, 128, 111,
            ],
    ).unwrap();

        (conn, conn2)
    }

    #[test]
    fn test_exchange_keys() {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(TEST_FILE_PATH)
            .unwrap();
        let file_clone = file.try_clone().unwrap();

        assert!(
            exchange_keys(Box::new(file), Box::new(file_clone), true, vec![1; 32].as_slice(), vec![2; 32].as_slice()).is_ok(),
            "A stream should be able to get created with the above config"
        );
        assert!(
            exchange_keys(Box::new(file), Box::new(file_clone), false, vec![1; 31].as_slice(), vec![2; 32].as_slice()).is_err(),
            "Key seed is too small, must be 32 bytes"
        );
        assert!(
            exchange_keys(Box::new(file), Box::new(file_clone), true, vec![1; 33].as_slice(), vec![2; 32].as_slice()).is_err(),
            "Key seed is too big, must be 32 bytes"
        );
        assert!(
            exchange_keys(Box::new(file), Box::new(file_clone), false, vec![1; 32].as_slice(), vec![2; 31].as_slice()).is_err(),
            "Remote key is too small, must be 32 bytes"
        );
        assert!(
            exchange_keys(Box::new(file), Box::new(file_clone), true, vec![1; 32].as_slice(), vec![2; 33].as_slice()).is_err(),
            "Remote key seed is too big, must be 32 bytes"
        );
    }

    #[test]
    fn chunk_dechunk() {
        // TODO: Test out-of-order and lost chunks
        let (client, server) = stream_prelude();
        let (mut client_in, mut client_out) = client;
        let (mut server_in, mut server_out) = server;

        let chunked = client_out.chunk(0, 0, &[1; 2]).unwrap();
        let data = server_in.dechunk().unwrap();

        client_out.packet_config.max_size = 100;
        server_out.packet_config.max_size = 100;

        test_stream_chunking(true, client_out.borrow_mut(), server_in.borrow_mut(), 0, 0, &[]);
        test_stream_chunking(true, server_out.borrow_mut(), client_in.borrow_mut(), 0, 0, &[]);

        client_out.packet_config.has_id = false;
        test_stream_chunking(
            true,
            client_out.borrow_mut(),
            server_in.borrow_mut(),
            13,
            8,
            &[3u8; 4],
        );
        test_stream_chunking(
            true,
            server_out.borrow_mut(),
            client_in.borrow_mut(),
            13,
            8,
            &[3u8; 4],
        );

        client_out.packet_config.has_data_len = false;
        test_stream_chunking(
            false,
            server_out.borrow_mut(),
            client_in.borrow_mut(),
            0x1FF_FFFF,
            8,
            &[4u8; 512],
        );
        test_stream_chunking(
            false,
            server_out.borrow_mut(),
            client_in.borrow_mut(),
            0x1_FFFF,
            0x1F,
            &[4u8; 512],
        );

        client_out.packet_config.has_sequence = false;
        test_stream_chunking(
            true,
            client_out.borrow_mut(),
            server_in.borrow_mut(),
            13,
            8,
            &[4u8; 512],
        );
        test_stream_chunking(
            true,
            server_out.borrow_mut(),
            client_in.borrow_mut(),
            13,
            8,
            &[4u8; 512],
        );
    }

    #[cfg_attr(tarpaulin, skip)]
    fn test_stream_chunking(
        succ: bool,
        a: &mut StreamOut,
        b: &mut StreamIn,
        id: u32,
        ch: u8,
        data: &[u8],
    ) {
        if let Err(_) = a.chunk(id, ch, &data) {
            return assert!(!succ, "Chunk should have been created!");
        }

        let dechunked = match b.dechunk() {
            Ok(d) => d,
            Err(_) => {
                return assert!(!succ, "Chunk should have been created!");
            }
        };

        if succ {
            assert_eq!(data, dechunked.data.as_slice());
            assert_eq!(ch, dechunked.get_channel());
            assert_eq!(id, dechunked.id);
        } else {
            assert_ne!(data, dechunked.data.as_slice());
            assert_ne!(ch, dechunked.get_channel());
            assert_ne!(id, dechunked.id);
        }
    }
}
