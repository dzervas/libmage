use std::io::{Error, ErrorKind, Read, Result, Write};

use super::error_str;
use crate::packet::{Packet, PacketConfig};

use c2_chacha::stream_cipher::{NewStreamCipher, SyncStreamCipher};
use c2_chacha::ChaCha20;
use rand::random;

pub struct StreamIn
{
    reader: Box<dyn Read + Send + Sync>,
    key: [u8; 32],
}

impl StreamIn
{
    pub fn new(reader: Box<dyn Read + Send + Sync>, key: [u8; 32]) -> Self {
        Self {
            key,
            reader,
        }
    }

    pub fn dechunk(&mut self) -> Result<Packet> {
        // TODO: Use c2_chacha constants
        let mut buffer: [u8; 256] = [0; 256];
        let mut nonce: [u8; 8] = [0; 8];

        self.reader.read_exact(&mut nonce)?;

        let mut cipher = match ChaCha20::new_var(&self.key, &nonce) {
            Ok(d) => d,
            Err(_e) => return Err(error_str!("Invalid nonce length!"))
        };

        // TODO: Handle ErrorKind::Interrupted | ErrorKind::UnexpectedEof | ErrorKind::WouldBlock gracefully
        let length = self.reader.read(&mut buffer)?;

        #[cfg(not(test))]
        println!("Dechunking {}: {:?}", length, &buffer[..length]);

        cipher.apply_keystream(&mut buffer[..length]);

        let (packet, _config, _deserialized_bytes) = PacketConfig::deserialize(&buffer[..length]);
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
    key: [u8; 32],
}

impl StreamOut
{
    pub fn new(writer: Box<dyn Write + Send + Sync>, key: [u8; 32]) -> Self {
        Self {
            packet_config: PacketConfig {
                has_id: true,
                has_sequence: true,
                has_data_len: true,
                max_size: 256,
            },
            key,
            writer,
        }
    }

    pub fn chunk(&mut self, id: u32, channel: u8, data: &[u8]) -> Result<()> {
        let mut i: u32 = 0;
        let mut chunked: usize = 0;

        let nonce: [u8; 8] = random();
        let mut cipher = match ChaCha20::new_var(&self.key, &nonce) {
            Ok(d) => d,
            Err(e) => return Err(error_str!(format!("Unable to initialize encrypt keystream: {:?}", e)))
        };

        self.writer.write_all(&nonce)?;

        while {
            let (mut plain, w) = self
                .packet_config
                .serialize(id, channel, i, &data[chunked..])?;

            chunked += w;
            i += 1;

            cipher.apply_keystream(&mut plain);

            #[cfg(not(test))]
            println!("Chunked {}: {:?}", plain.len(), plain);

            self.writer.write_all(plain.as_slice())?;

            // NOTE: This is do..while, check https://gist.github.com/huonw/8435502
            chunked < data.len()
        } {}

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::OpenOptions;

    // Known keys: vec![1; 32] -> public vec![171, 47, 202, 50, 137, 131, 34, 194, 8, 251, 45, 171, 80, 72, 189, 67, 195, 85, 198, 67, 15, 88, 136, 151, 203, 87, 73, 97, 207, 169, 128, 111]
    // Known keys: vec![2; 32] -> public vec![252, 59, 51, 147, 103, 165, 34, 93, 83, 169, 45, 56, 3, 35, 175, 208, 53, 215, 129, 123, 109, 27, 228, 125, 148, 111, 107, 9, 169, 203, 220, 6]

    #[cfg(target_os = "windows")]
    const TEST_FILE_PATH: &str = ".mage-test.tmp";

    #[cfg(not(target_os = "windows"))]
    const TEST_FILE_PATH: &str = ".mage-test.tmp";

    macro_rules! open_test_file {
        () => {
            OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(TEST_FILE_PATH)
                .unwrap()
        };
    }

    #[cfg(not(tarapaulin_include))]
    fn stream_prelude() -> ((StreamIn, StreamOut), (StreamIn, StreamOut)) {
        let file = open_test_file!();
        let file_clone = file.try_clone().unwrap();

        let conn_in = StreamIn::new(Box::new(file), [1; 32]);
        let conn_out = StreamOut::new(Box::new(file_clone), [1; 32]);

        let file2 = open_test_file!();
        let file2_clone = file2.try_clone().unwrap();

        let conn2_in = StreamIn::new(Box::new(file2), [2; 32]);
        let conn2_out = StreamOut::new(Box::new(file2_clone), [2; 32]);

        ((conn_in, conn_out), (conn2_in, conn2_out))
    }

    // TODO: Test this using a network protocol, plain file does not work
    // #[test]
    #[allow(dead_code)]
    fn chunk_dechunk() {
        // TODO: Test out-of-order and lost chunks
        let (client, server) = stream_prelude();
        let (mut client_in, mut client_out) = client;
        let (mut server_in, mut server_out) = server;

        let _chunked = client_out.chunk(0, 0, &[1; 2]).unwrap();
        let _data = server_in.dechunk().unwrap();

        client_out.packet_config.max_size = 100;
        server_out.packet_config.max_size = 100;

        test_stream_chunking(true, &mut client_out, &mut server_in, 0, 0, &[]);
        test_stream_chunking(true, &mut server_out, &mut client_in, 0, 0, &[]);

        client_out.packet_config.has_id = false;
        test_stream_chunking(
            true,
            &mut client_out,
            &mut server_in,
            13,
            8,
            &[3u8; 4],
        );
        test_stream_chunking(
            true,
            &mut server_out,
            &mut client_in,
            13,
            8,
            &[3u8; 4],
        );

        client_out.packet_config.has_data_len = false;
        test_stream_chunking(
            false,
            &mut server_out,
            &mut client_in,
            0x1FF_FFFF,
            8,
            &[4u8; 512],
        );
        test_stream_chunking(
            false,
            &mut server_out,
            &mut client_in,
            0x1_FFFF,
            0x1F,
            &[4u8; 512],
        );

        client_out.packet_config.has_sequence = false;
        test_stream_chunking(
            true,
            &mut client_out,
            &mut server_in,
            13,
            8,
            &[4u8; 512],
        );
        test_stream_chunking(
            true,
            &mut server_out,
            &mut client_in,
            13,
            8,
            &[4u8; 512],
        );
    }

    #[cfg(not(tarapaulin_include))]
    fn test_stream_chunking(
        succ: bool,
        a: &mut StreamOut,
        b: &mut StreamIn,
        id: u32,
        ch: u8,
        data: &[u8],
    ) {
        if let Err(e) = a.chunk(id, ch, &data) {
            return assert!(!succ, format!("Chunk should have been created! {:?}", e));
        }

        let dechunked = match b.dechunk() {
            Ok(d) => d,
            Err(e) => {
                return assert!(!succ, format!("Chunk should have been created! {:?}", e));
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
