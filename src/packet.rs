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
		result |= (self.seq_len << 4) | (self.data_len_len << 2) | self.id_len;

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
			seq_len: (data[1] & 0b110000) >> 4,
			data_len_len: (data[1] & 0b1100) >> 2,
			id_len: (data[1] & 0b11)
		};
		let offset = 2 + config.seq_len + config.data_len_len + config.id_len;
		let sequence: u32 = bytes_to_u32(&data[2..(2 + config.seq_len) as usize]);
		let data_len: u32 = bytes_to_u32(&data[(2 + config.seq_len) as usize..(offset - config.id_len) as usize]);
		let id: u32 = bytes_to_u32(&data[(offset - config.id_len) as usize..offset as usize]);

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