//extern crate serde_derive;

//use self::serde_derive::{Deserialize, Serialize};

#[derive(PartialEq, Debug)]
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

pub fn serialize(&self) -> u8 {
	let mut result: u8 = 0;
	// error handling?

	if self.first { result |= 1 << 7 }
	if self.last { result |= 1 << 6 }
	result |= (self.seq_len << 4) | (self.data_len_len << 2) | self.id_len;

	result
}
}

#[derive(PartialEq, Debug)]
pub struct Packet<'a> {
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
	config: Config,

	// --------- Length --------- 0-6 Bytes - Little Endian!
	// 3 Byte : Agent ID (optional, up to 3 bytes)
	pub id: u32,
	// 3 Bytes: Sequence number (optional, up to 3 bytes)
	sequence: u32,
	// 3 Bytes: Data length (optional, up to 3 bytes)
	pub data_len: u32,

	// --------- Data --------- N Bytes
	data: &'a [u8],
}

impl Packet<'_> {
	pub fn new(channel: u8, data: &[u8]) -> Packet {
		// error handling??

		Packet {
			channel,
			config: Config::new(),
			id: 0,
			sequence: 0,
			data_len: 0,
			data,
		}
	}

	fn calculate_lengths(&mut self) {
		// error handling?
        self.data_len = self.data.len() as u32;

		if self.config.id_len > 0 && self.id > 0xFF {
			self.config.id_len = 2;
		} else if self.config.id_len > 0 {
			self.config.id_len = 1;
		}

		if self.config.data_len_len > 0 && self.data_len > 0xFF {
			self.config.data_len_len = 2;
		} else if self.config.data_len_len > 0 {
			self.config.data_len_len = 1;
		}

		if self.config.seq_len > 0 && self.sequence > 0xFF {
			self.config.seq_len = 2;
		} else if self.config.seq_len > 0 {
			self.config.seq_len = 1;
		}
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

	pub fn serialize(&mut self) -> Vec<u8> {
		// Hardcoded protocol version
		let mut result: Vec<u8> = vec![0x00 | self.channel];

        self.calculate_lengths();
		result.push(self.config.serialize());

		for i in (0..self.config.id_len).rev() {
			result.push((self.id >> (i as u32)*8) as u8);
		}

		for i in (0..self.config.seq_len).rev() {
			result.push((self.sequence >> (i as u32)*8) as u8);
		}

		for i in (0..self.config.data_len_len).rev() {
			result.push((self.data_len >> (i as u32)*8) as u8);
		}

		result.append(self.data.to_vec().as_mut());

		result.clone()
	}
}

pub fn deserialize(data: &[u8] ) -> Packet {
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


	// error handling

	Packet{
		channel,
		config,
		data: &data[offset as usize..data.len() as usize],
		sequence,
		data_len,
		id
	}
}

fn bytes_to_u32(bytes: &[u8]) -> u32 {
	let mut buf: u32 = 0;
	if bytes.len() > 0 { buf |= (bytes[0] as u32) << 24 }
	if bytes.len() > 1 { buf |= (bytes[1] as u32) << 16 }
	if bytes.len() > 2 { buf |= (bytes[2] as u32) << 8 }
	if bytes.len() > 3 { buf |= (bytes[3] as u32) }
	buf
}