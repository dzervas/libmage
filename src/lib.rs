#![feature(new_uninit)]
#![feature(get_mut_unchecked)]
#![feature(maybe_uninit_extra)]
#![feature(maybe_uninit_ref)]
extern crate custom_error;
extern crate crossbeam_channel;
extern crate sodiumoxide;

pub mod packet;
pub mod stream;
pub mod connection;
pub mod channel;

extern crate lazy_static;

use std::net::TcpStream;
use std::io::{Read, Write};
use std::os::raw::{c_int, c_void};
use connection::Connection;
use std::slice::{from_raw_parts, from_raw_parts_mut};
use std::sync::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    static ref CONN: Mutex<Option<Box<Connection<'static>>>> = Mutex::new(None);
}

//static mut trans: Option<&TcpStream> = None;

// TODO: bind, listen, accept
#[no_mangle]
pub unsafe extern fn connect(_socket: c_int, _sockaddr: *const c_void, _address_len: c_void) -> c_int {
    let stream: TcpStream = TcpStream::connect("127.0.0.1:4444").unwrap();
//    static mut reader: Rc<dyn Read> = Rc::new(stream.try_clone().unwrap());
//    static mut writer: Rc<dyn Write> = Rc::new(stream.try_clone().unwrap());
//    let mut reader: dyn Read + Send = stream.try_clone().unwrap();
//    let mut writer: dyn Write + Send = stream.try_clone().unwrap();
//    let c = Connection::new(0, &mut reader, &mut writer, false, &[1; 32], &[252, 59, 51, 147, 103, 165, 34, 93, 83, 169, 45, 56, 3, 35, 175, 208, 53, 215, 129, 123, 109, 27, 228, 125, 148, 111, 107, 9, 169, 203, 220, 6]).unwrap();
    let c = Connection::new_box(0, Box::new(stream.try_clone().unwrap()), Box::new(stream.try_clone().unwrap()), false, &[1; 32], &[252, 59, 51, 147, 103, 165, 34, 93, 83, 169, 45, 56, 3, 35, 175, 208, 53, 215, 129, 123, 109, 27, 228, 125, 148, 111, 107, 9, 169, 203, 220, 6]).unwrap();
    let mut conn = CONN.lock().unwrap();

    *conn = Some(Box::new(c));
//    replace(&mut *conn, Mutex::new(Some(c)));

    0
}

#[no_mangle]
pub unsafe extern fn send(_socket: c_int, msg: *const c_void, size: usize, _flags: c_int) -> usize {
    let buf = from_raw_parts(msg as *const u8, size);
//    CONN.lock().unwrap().write(buf).unwrap()
    0
}

#[no_mangle]
pub unsafe extern fn recv(_socket: c_int, msg: *mut c_void, size: usize, _flags: c_int) -> usize {
    let buf = from_raw_parts_mut(msg as *mut u8, size);
//    CONN.lock().unwrap().read(buf).unwrap()
    0
}
//#[no_mangle]
//pub unsafe extern "C" fn flush(s: &mut connection::Connection) { conn.unwrap().flush().unwrap() }
