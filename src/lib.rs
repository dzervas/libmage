#![feature(thread_local)]
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
use std::os::raw::{c_void, c_int};
use connection::Connection;
use std::slice::{from_raw_parts, from_raw_parts_mut};
use lazy_static::lazy_static;
use std::sync::Mutex;

lazy_static!{
//    static STREAM: Mut<TcpStream> = Rc::new(RefCell::new());
    static ref CONN: Mutex<Connection> = {
        let tcpstream = TcpStream::connect("127.0.0.1:4444").unwrap();
        let conn = Connection::new(0, Box::new(tcpstream), false, &[1; 32], &[252, 59, 51, 147, 103, 165, 34, 93, 83, 169, 45, 56, 3, 35, 175, 208, 53, 215, 129, 123, 109, 27, 228, 125, 148, 111, 107, 9, 169, 203, 220, 6]).unwrap();

        Mutex::new(conn)
    };
}

// TODO: bind, listen, accept
#[no_mangle]
pub unsafe extern fn connect(_socket: c_int, _sockaddr: *const c_void, _address_len: c_void) -> c_int {
//    let stream: TcpStream = TcpStream::connect("127.0.0.1:4444").unwrap();
//    let c = Connection::new(0, &mut stream.try_clone().unwrap(), &mut stream.try_clone().unwrap(), false, &[1; 32], &[252, 59, 51, 147, 103, 165, 34, 93, 83, 169, 45, 56, 3, 35, 175, 208, 53, 215, 129, 123, 109, 27, 228, 125, 148, 111, 107, 9, 169, 203, 220, 6]).unwrap();
//    let c = Connection::new_box(0, Box::new(stream.try_clone().unwrap()), Box::new(stream.try_clone().unwrap()), false, &[1; 32], &[252, 59, 51, 147, 103, 165, 34, 93, 83, 169, 45, 56, 3, 35, 175, 208, 53, 215, 129, 123, 109, 27, 228, 125, 148, 111, 107, 9, 169, 203, 220, 6]).unwrap();

//    CONN.with(|conn| Rc::into_raw(*conn) as *mut c_void)
    0
}

#[no_mangle]
pub unsafe extern fn send(_socket: c_int, msg: *const c_void, size: usize, _flags: c_int) -> usize {
    let buf = from_raw_parts(msg as *const u8, size);
    CONN.lock().unwrap().write(buf).unwrap()
//    0
}

#[no_mangle]
pub unsafe extern fn recv(_socket: c_int, msg: *mut c_void, size: usize, _flags: c_int) -> usize {
    let buf = from_raw_parts_mut(msg as *mut u8, size);
    CONN.lock().unwrap().read(buf).unwrap()
//    0
}
//#[no_mangle]
//pub unsafe extern "C" fn flush(s: &mut connection::Connection) { conn.unwrap().flush().unwrap() }
