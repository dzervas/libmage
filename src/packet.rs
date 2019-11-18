use std::cmp::Ordering;

#[derive(Debug)]
pub enum ConfigError {
	IdOverflow,
	SequenceOverflow,
	DataLenOverflow,
}

#[derive(Eq, Copy, Clone, PartialEq, Debug)]
pub struct Config {
	pub first: bool,
	pub last: bool,
	pub seq_len: u8, // 2 bits
	pub data_len_len: u8, // 2 bits
	pub id_len: u8, // 2 bits
}

impl Config {
	pub fn new() -> Config {
		Config {
			first: false,
			last: false,
			seq_len: 0,
			data_len_len: 0,
			id_len: 0
		}
	}

	pub fn serialize(&self) -> Result<u8, ConfigError> {
		let mut result: u8 = 0;

		if self.id_len > 0b11 { return Err(ConfigError::IdOverflow) }
		if self.seq_len > 0b11 { return Err(ConfigError::SequenceOverflow) }
		if self.data_len_len > 0b11 { return Err(ConfigError::DataLenOverflow) }

		if self.first { result |= 1 << 7 }
		if self.last { result |= 1 << 6 }
		result |= (self.id_len << 4) | (self.seq_len << 2) | self.data_len_len;

		Ok(result)
	}
}

#[derive(Debug)]
pub enum PacketError {
	ChannelOverflow,
	IdOverflow,
	SequenceOverflow,
	DataLenOverflow,
	ConfigSerializationError,
}

#[derive(Eq, Clone, Debug)]
pub struct Packet {
	// --------- Channel & Version --------- 1 Byte
	// The protocol version is crammed in here during serialization
	// and removed during deserialization
	// 4 Bits : Protocol version - not accessible
	// 4 Bits : Channel ID
	pub channel: u8,

	// --------- Config --------- 1 Byte
	// 1 Bit  : Is this the first packet
	// 1 Bit  : Is this the last packet
	// 2 Bits : Length of sequence number
	// 2 Bits : Length of data length field
	// 2 Bits : Length of ID
	pub config: Config,

	// --------- Length --------- 0-6 Bytes - Little Endian!
	// 3 Byte : Agent ID (optional, up to 3 bytes)
	pub id: u32,
	// 3 Bytes: Sequence number (optional, up to 3 bytes)
	pub sequence: u32,
	// 3 Bytes: Data length (optional, up to 3 bytes)
	pub data_len: u32,

	// --------- Data --------- N Bytes
	pub data: Vec<u8>,
}

impl Packet {
	pub fn new(channel: u8, id: u32, sequence: u32, data: Vec<u8>) -> Result<Self, PacketError> {
		// Check correct field lengths. Check struct definition
		if channel > 0xf { return Err(PacketError::ChannelOverflow) }
		if id > 0xffffff { return Err(PacketError::IdOverflow) }
		if sequence > 0xffffff { return Err(PacketError::SequenceOverflow) }
		if data.len() > 0xffffff { return Err(PacketError::DataLenOverflow) }

		Ok(Packet {
			channel,
			config: Config::new(),
			id,
			sequence,
			data_len: 0,
			data,
		})
	}

	pub fn calculate_lengths(&mut self) -> Result<usize, PacketError> {
		// Check correct field lengths. Check struct definition
		if self.channel > 0xf { return Err(PacketError::ChannelOverflow) }
		if self.id > 0xffffff { return Err(PacketError::IdOverflow) }
		if self.sequence > 0xffffff { return Err(PacketError::SequenceOverflow) }
		if self.data.len() > 0xffffff { return Err(PacketError::DataLenOverflow) }

		self.data_len = self.data.len() as u32;

		let calc_bytes = |x| {
			if x <= 0 { return 1u8 }
			(x as f64).log(0x100 as f64).ceil() as u8
		};

		if self.config.id_len > 0 { self.config.id_len = calc_bytes(self.id); }
		if self.config.data_len_len > 0 { self.config.data_len_len = calc_bytes(self.data_len); }
		if self.config.seq_len > 0 { self.config.seq_len = calc_bytes(self.sequence); }

		Ok((2 + self.config.id_len + self.config.seq_len + self.config.data_len_len) as usize)
	}

	pub fn has_id(&mut self, v: bool) {
		if v { self.config.id_len = 3 }
		else { self.config.id_len = 0 }
	}

	pub fn has_data_len(&mut self, v: bool) {
		if v { self.config.data_len_len = 3 }
		else { self.config.data_len_len = 0 }
	}

	pub fn has_sequence(&mut self, v: bool) {
		if v { self.config.seq_len = 3 }
		else { self.config.seq_len = 0 }
	}

	pub fn first(&mut self, v: bool) {
		self.config.first = v;
	}

	pub fn last(&mut self, v: bool) {
		self.config.last = v;
	}

	pub fn serialize(&mut self) -> Result<Vec<u8>, PacketError> {
		// Hardcoded protocol version  vvv
		let mut result: Vec<u8> = vec![0x00 | self.channel];

		match self.config.serialize() {
			Ok(d) => result.push(d),
			Err(_) => return Err(PacketError::ConfigSerializationError)
		}

		for i in (0..self.config.id_len).rev() {
			result.push(((self.id >> (8*i as u32)) & 0xff) as u8);
		}

		for i in (0..self.config.seq_len).rev() {
			result.push((self.sequence >> (i as u32)*8) as u8);
		}

		for i in (0..self.config.data_len_len).rev() {
			result.push((self.data_len >> (i as u32)*8) as u8);
		}

		result.append(self.data.to_vec().as_mut());

		Ok(result.clone())
	}

	pub fn deserialize(data: &[u8] ) -> Result<Self, PacketError> {
		let channel = data[0];
		let config = Config{
			first: (data[1] & (1 << 7)) > 0,
			last: (data[1] & (1 << 6)) > 0,
			id_len: (data[1] & 0b110000) >> 4,
			seq_len: (data[1] & 0b1100) >> 2,
			data_len_len: data[1] & 0b11,
		};
		let offset = 2 + config.id_len + config.seq_len + config.data_len_len;
		let id: u32 = bytes_to_u32(&data[2..(2 + config.id_len) as usize]);
		let sequence: u32 = bytes_to_u32(&data[(2 + config.id_len) as usize..(2 + config.id_len + config.seq_len) as usize]);
		let data_len: u32 = bytes_to_u32(&data[(2 + config.id_len + config.seq_len) as usize..offset as usize]);

		// Can't detect errors here... NaCl should check for errors (?)
        // Return Ok() for consistency

		Ok(Packet{
			channel,
			config,
			data: data[offset as usize..data.len() as usize].to_vec(),
			sequence,
			data_len,
			id
		})
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
		self.sequence == other.sequence
	}
}

fn bytes_to_u32(bytes: &[u8]) -> u32 {
	let mut buf: u32 = 0;

	for i in {0..bytes.len()} {
		buf <<= 8;
		buf |= bytes[i] as u32;
	}

	buf
}


#[cfg(test)]
mod tests {
	use super::*;

	#[test]
    fn config() {
		let mut config = Config::new();
		assert_eq!(config.serialize().unwrap(), 0);

		config.first = true;
		assert_eq!(config.serialize().unwrap(), 0b10000000);

		config.last = true;
		assert_eq!(config.serialize().unwrap(), 0b11000000);

		config.data_len_len = 3;
		assert_eq!(config.serialize().unwrap(), 0b11000011);

		config.id_len = 0x3f;
		assert!(config.serialize().is_err(), "ID Length should be 2 bits (<4)");
    }

	#[test]
	fn packet() {
        assert!(Packet::new(1, 1234, 0, vec![2u8; 2]).is_ok(), "A packet should be able to get created with the above config");
		assert!(Packet::new(0x1f, 1234, 0, vec![2u8; 2]).is_err(), "Channel should be 4 bits (<16)");
		assert!(Packet::new(0xf, 0x1ffffff, 0, vec![2u8; 2]).is_err(), "ID should be 3 bytes (<0xFFFFFF)");
		assert!(Packet::new(0xf, 0xffff, 0x1ffffff, vec![2u8; 2]).is_err(), "Sequence should be 3 bytes (<0xFFFFFF)");
		assert!(Packet::new(0xf, 0xffff, 0xffff, vec![2u8; 0x1ffffff]).is_err(), "Data length should be 3 bytes (<0xFFFFFF)");
	}

	#[test]
	fn serialize_deserialize() {
		let mut p = Packet::new(1, 0x1234, 7, vec![2u8; 3]).unwrap();
		p.has_id(true);
		p.has_data_len(true);
        p.has_sequence(true);

		p.calculate_lengths();

		let s = p.serialize().unwrap();
        assert_eq!(s, vec![1, 0b00100101, 0x12, 0x34, 7, 3, 2, 2, 2]);

		let d = Packet::deserialize(s.as_slice()).unwrap();
		assert_eq!(p, d);

        // On purpose does not call calculate_lengths
		p.config.id_len = 10;
		assert!(p.serialize().is_err(), "Config has invalid ID Length");
	}

	#[test]
	fn bytes_u32() {
		assert_eq!(bytes_to_u32(vec![1].as_slice()), 1u32);
		assert_eq!(bytes_to_u32(vec![0x10, 0xff].as_slice()), 0x10ff);
		assert_eq!(bytes_to_u32(vec![0x12, 0x34, 0x56, 0x78].as_slice()), 0x12345678);
	}
}