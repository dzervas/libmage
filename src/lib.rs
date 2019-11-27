#[macro_use]
extern crate custom_error;
extern crate crossbeam_channel;
extern crate sodiumoxide;

pub mod packet;
pub mod stream;
pub mod connection;
pub mod channel;

//use std::io::{Read, Write};
//use std::io;

//type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

//#[no_mangle]
//pub extern "C" fn connection_new<'conn>(id: u32, reader: &'conn mut impl Read, writer: &'conn mut impl Write, server: bool, seed: &[u8], remote_key: &[u8]) -> Result<connection::Connection<'conn>> {
//    connection::Connection::new(id, reader, writer, server, seed, remote_key)
//}
//#[no_mangle]
//pub extern "C" fn connection_write(s: &mut connection::Connection, buf: &[u8]) -> io::Result<usize> { s.write(buf) }
//#[no_mangle]
//pub extern "C" fn connection_read(s: &mut connection::Connection, mut buf: &mut [u8]) -> io::Result<usize> { s.read(buf) }
//#[no_mangle]
//pub extern "C" fn connection_flush(s: &mut connection::Connection) -> io::Result<()> { s.flush() }
