use std::cmp::Ordering;
use std::io::{Error, ErrorKind, Result};

use super::error_str;

#[derive(Eq, Debug)]
pub struct Packet {
    // The protocol version is crammed in here during serialization
    // and removed during deserialization
    // 4 Bits : Protocol version - not accessible
    // 4 Bits : Channel ID
    channel: u8,
    // 1 Byte : PacketConfig
    // 3 Byte : Agent ID (optional, up to 3 bytes)
    pub id: u32,
    // 3 Bytes: Sequence number (optional, up to 3 bytes)
    pub sequence: u32,
    // N byte
    pub data: Vec<u8>,
}

impl Packet {
    #[allow(dead_code)]
    pub fn get_channel(&self) -> u8 {
        self.channel & 0xF
    }
    #[allow(dead_code)]
    pub fn get_version(&self) -> u8 {
        self.channel >> 4
    }
}

impl Ord for Packet {
    fn cmp(&self, other: &Self) -> Ordering {
        self.sequence.cmp(&other.sequence)
    }
}

impl PartialOrd for Packet {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.sequence.partial_cmp(&other.sequence)
    }
}

impl PartialEq for Packet {
    fn eq(&self, other: &Self) -> bool {
        self.channel == other.channel
            && self.id == other.id
            && self.sequence == other.sequence
            && self.data == other.data
    }
}

#[derive(Eq, Copy, Clone, PartialEq, Debug)]
pub struct PacketConfig {
    // --------- Config --------- 1 Byte
    // 2 Bits : Reserved
    // 2 Bits : Length of data length field
    // 2 Bits : Length of sequence number
    // 2 Bits : Length of ID
    pub has_id: bool,
    pub has_sequence: bool,
    pub has_data_len: bool,

    // Max packet length
    pub max_size: usize,
}

impl PacketConfig {
    pub fn serialize(
        &self,
        id: u32,
        channel: u8,
        sequence: u32,
        data: &[u8],
    ) -> Result<(Vec<u8>, usize)> {
        #[allow(clippy::identity_op)]
        // This is to remind that 0x00 is the version and can be changed
        // Hardcoded protocol version  vvv - only the left part should change! 0x10, 0x20...
        let mut result: Vec<u8> = vec![0x00 | channel];

        if channel > 0xF {
            return Err(error_str!(
                "Field channel is more than half a byte. It must have a value <= 0xF"
            ));
        }

        let id_len = calculate_byte_length(self.has_id, id, "id")?;
        let seq_len = calculate_byte_length(self.has_sequence, sequence, "sequence")?;

        let overhead = 2 + id_len as usize + seq_len as usize;
        let mut data_len = if data.len() < self.max_size - overhead {
            data.len()
        } else {
            self.max_size - overhead
        };
        let mut data_len_len =
            calculate_byte_length(self.has_data_len, data_len as u32, "data_len")?;

        // Recalculate data length to fit data in max_size
        while overhead + data_len + data_len_len as usize > self.max_size {
            data_len -= 1;
            data_len_len = calculate_byte_length(self.has_data_len, data_len as u32, "data_len")?;

            if data_len == 0 {
                return Err(error_str!(
                    "Field data_len is more than 3 bytes. It must be 3 bytes (<=0xFF_FFFF) max"
                ));
            }
        }

        result.push((id_len << 4) | (seq_len << 2) | data_len_len);

        for i in (0..id_len).rev() {
            result.push((id >> ((i as u32) * 8)) as u8);
        }

        for i in (0..seq_len).rev() {
            result.push((sequence >> ((i as u32) * 8)) as u8);
        }

        for i in (0..data_len_len).rev() {
            result.push((data_len as u32 >> ((i as u32) * 8)) as u8);
        }

        result.extend(&data[..data_len]);

        Ok((result, data_len))
    }

    pub fn deserialize(data: &[u8]) -> (Packet, Self, usize) {
        // let version = data[0] & 0xF0 // Should implement version check at some point
        let channel = data[0] & 0xF;

        let id_len = (data[1] & 0b11_0000) >> 4;
        let seq_len = (data[1] & 0b1100) >> 2;
        let data_len_len = data[1] & 0b11;
        let offset = 2 + id_len + seq_len + data_len_len;

        let id = bytes_to_u32(&data[2..(2 + id_len) as usize]);
        let sequence = bytes_to_u32(&data[(2 + id_len) as usize..(2 + id_len + seq_len) as usize]);
        let data_len = if data_len_len > 0 {
            bytes_to_u32(&data[(2 + id_len + seq_len) as usize..offset as usize])
        } else {
            data.len() as u32 - offset as u32
        };

        (
            Packet {
                id,
                channel,
                sequence,
                data: data[offset as usize..offset as usize + data_len as usize].to_vec(),
            },
            Self {
                has_id: id_len > 0,
                has_sequence: seq_len > 0,
                has_data_len: data_len_len > 0,
                max_size: data_len as usize + offset as usize,
                // max_size is the same with the last value in tuple. maybe remove it?
            },
            data_len as usize + offset as usize,
        )
    }
}

fn calculate_byte_length(has_length: bool, value: u32, field_name: &'static str) -> Result<u8> {
    // While logarithm is "the right way", ifs are much faster
    // if x <= 0 { return 1u8 }
    // (x as f64).log(0x100 as f64).ceil() as u8

    if !has_length {
        Ok(0)
    } else if value > 0xFF_FFFF {
        Err(error_str!(
            "Field {} is more than 3 bytes. It must be 3 bytes (<=0xFF_FFFF) max",
            field_name
        ))
    } else if value > 0xFFFF {
        Ok(3)
    } else if value > 0xFF {
        Ok(2)
    } else {
        Ok(1)
    }
}

fn bytes_to_u32(bytes: &[u8]) -> u32 {
    let mut buf: u32 = 0;

    for byte in bytes.iter() {
        buf <<= 8;
        buf |= *byte as u32;
    }

    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packet() {
        let mut pc = PacketConfig {
            has_id: true,
            has_sequence: true,
            has_data_len: true,
            max_size: 256usize,
        };
        let (p1, _) = pc.serialize(0x1234, 1, 1, &[2u8; 3]).unwrap();
        let (p2, _) = pc.serialize(0x1234, 1, 2, &[2u8; 3]).unwrap();
        let (p3, _) = pc.serialize(0x1234, 1, 2, &[2u8; 3]).unwrap();

        pc = PacketConfig {
            has_id: true,
            has_sequence: true,
            has_data_len: false,
            max_size: 256usize,
        };
        let (p4, _) = pc.serialize(0, 1, 7, &[2u8; 3]).unwrap();

        // Test config overflows
        assert!(
            pc.serialize(0x1FF_FFFF, 1, 1, &[2u8; 1]).is_err(),
            "ID should be <= 3 bytes length (<=0xFF_FFFF)"
        );
        assert!(
            pc.serialize(1, 0x1F, 1, &[2u8; 1]).is_err(),
            "Channel should be <= 4 bits length (<=0xF)"
        );
        assert!(
            pc.serialize(1, 1, 0x1FF_FFFF, &[2u8; 1]).is_err(),
            "Sequence should be <= 3 bytes length (<=0xFF_FFFF)"
        );
        let (d, dl) = pc.serialize(1, 1, 1, &[2u8; 0x1FF_FFFF]).unwrap();
        assert_eq!(d.len(), pc.max_size);
        assert!(
            dl < pc.max_size,
            "Data Length should be <= 3 bytes length (<=0xFF_FFFF)"
        );

        // Test serialized equality
        assert_eq!(p2, p3);
        assert_ne!(p1, p2);

        let (pd1, pd, pdl) = PacketConfig::deserialize(p1.as_slice());
        assert!(
            pdl < pc.max_size,
            "Data Length should be <= initial PacketConfig max_size"
        );
        let (pd2, _, _) = PacketConfig::deserialize(p2.as_slice());
        let (pd3, _, _) = PacketConfig::deserialize(p3.as_slice());
        let (pd4, _, _) = PacketConfig::deserialize(p4.as_slice());

        assert_eq!(pd1.get_channel(), 1);
        assert_eq!(pd2.get_channel(), 1);
        assert_eq!(pd3.get_channel(), 1);
        assert_eq!(pd4.get_channel(), 1);

        assert_eq!(pd1.get_version(), 0);
        assert_eq!(pd2.get_version(), 0);
        assert_eq!(pd3.get_version(), 0);
        assert_eq!(pd4.get_version(), 0);

        // Test deserialized equality
        assert_eq!(pd2, pd3);
        assert_ne!(pd1, pd2);

        // Test order
        assert!(
            pd2 > pd1,
            "p2 is greater than p1 because it has higher sequence"
        );

        assert_eq!(pd1.cmp(&pd4), Ordering::Less);
        assert_eq!(pd2.cmp(&pd1), Ordering::Greater);
        assert_eq!(pd2.cmp(&pd3), Ordering::Equal);

        let (ps1, _) = pd.serialize(0x1234, 1, 1, &[2u8; 3]).unwrap();

        // Test serialize & deserialize
        assert_eq!(p1, ps1);
    }

    #[test]
    fn bytes_u32() {
        assert_eq!(bytes_to_u32(vec![1].as_slice()), 1u32);
        assert_eq!(bytes_to_u32(vec![0x10, 0xff].as_slice()), 0x10ff);
        assert_eq!(
            bytes_to_u32(vec![0x12, 0x34, 0x56, 0x78].as_slice()),
            0x1234_5678
        );
    }
}
