use std::io::{Read, Write};
use std::os::raw::{c_void, c_int};
use connection::Connection;
use std::slice::{from_raw_parts, from_raw_parts_mut};
use lazy_static::lazy_static;
use std::sync::Mutex;
use libc::ssize_t;

use transport;
use transport::Connector;

const ADDRESS: &str = "127.0.0.1:4444";

lazy_static!{
    static ref CONN: Mutex<Option<Connection>> = Mutex::new(None);
}

// TODO: bind, listen, accept
#[no_mangle]
pub extern fn connect(_socket: c_int, _sockaddr: *const c_void, _address_len: *mut c_void) -> c_int {
    // TODO: Choose transport (idea is to have many) using compile-time flags (or config file for server)
    let tcpstream = transport::tcp::Tcp::connect(ADDRESS).unwrap();
    let conn = Connection::new(0, Box::new(tcpstream), false, &[1; 32], &[252, 59, 51, 147, 103, 165, 34, 93, 83, 169, 45, 56, 3, 35, 175, 208, 53, 215, 129, 123, 109, 27, 228, 125, 148, 111, 107, 9, 169, 203, 220, 6]).unwrap();

    let mut conn_mut = CONN.lock().unwrap();
    *conn_mut = Some(conn);

    0
}

#[no_mangle]
pub unsafe extern fn send(_socket: c_int, msg: *const c_void, size: usize, _flags: c_int) -> ssize_t {
    let buf = from_raw_parts(msg as *const u8, size);
    CONN.lock()  // Mutex
        .unwrap().as_mut()  // MutexGuard (??)
        .unwrap()  // Option
        .write(buf).unwrap() as isize
}

#[no_mangle]
pub unsafe extern fn recv(_socket: c_int, msg: *mut c_void, size: usize, _flags: c_int) -> ssize_t {
    let buf = from_raw_parts_mut(msg as *mut u8, size);
    CONN.lock()  // Mutex
        .unwrap().as_mut()  // MutexGuard (??)
        .unwrap()  // Option
        .read(buf).unwrap() as isize
}

//#[no_mangle]
//pub extern fn bind(_socket: c_int, _sockaddr: *const c_void, _address_len: *mut c_void) -> c_int {
    //0
//}
//

#[no_mangle]
pub extern fn listen(_socket: c_int, _backlog: c_int) -> c_int {
    0
}

#[no_mangle]
pub extern fn accept(_socket: c_int, _sockaddr: *const c_void, _address_len: *mut c_void) -> c_int {
    0
}

//#[no_mangle]
//pub unsafe extern fn flush(s: &mut connection::Connection) { CONN.lock().unwrap().flush().unwrap() }
