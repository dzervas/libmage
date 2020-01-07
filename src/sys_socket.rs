use std::io::{Read, Write};
use std::os::raw::{c_void, c_int};
use connection::Connection;
use std::slice::{from_raw_parts, from_raw_parts_mut};
use lazy_static::lazy_static;
use std::sync::Mutex;
use libc::ssize_t;

use transport;
use transport::{Connector, Listener};

// TODO: These should be cfg-ish. Friendlier config.
const ADDRESS: &'static str = "127.0.0.1:4444";
type CONNECTOR = transport::tcp::Tcp;
type LISTENER = transport::tcp::Tcp;

const REMOTE_KEY: &[u8] = &[252, 59, 51, 147, 103, 165, 34, 93, 83, 169, 45, 56, 3, 35, 175, 208, 53, 215, 129, 123, 109, 27, 228, 125, 148, 111, 107, 9, 169, 203, 220, 6];
const SEED_KEY: &[u8] = &[1; 32];

// When returning a file descriptor (connect/listen/etc.) we need to return something
// distinct. Not an open file descriptor. This is achieved using luck (TM) that
// tells me that there won't be <BASE> open files by the target program.
// In order to handle multiple connections/listeners, an index is added to the
// base. This is used to map the "fd" (thinks the program) to our internal connection
// & listening vector.
const BASE_SOCKET_FD: c_int = 1000;
const BASE_ACCEPT_FD: c_int = BASE_SOCKET_FD + 1000;

lazy_static!{
    static ref SOCKET: Mutex<Vec<Connection>> = Mutex::new(vec![]);
    static ref ACCEPT: Mutex<Vec<LISTENER>> = Mutex::new(vec![]);
}

#[no_mangle]
pub extern fn connect(_socket: c_int, _sockaddr: *const c_void, _address_len: *mut c_void) -> c_int {
    let mut socket_mut = SOCKET.lock().unwrap();
    if BASE_SOCKET_FD + socket_mut.len() as c_int >= BASE_ACCEPT_FD {
        return -1 // Maybe we should cycle? -1 is erroneous state and programs know it
    }

    // TODO: Choose transport (idea is to have many) using compile-time flags (or config file for server)
    let new_conn = CONNECTOR::connect(ADDRESS).unwrap();
    let new_socket = Connection::new(0, Box::new(new_conn), false, SEED_KEY, REMOTE_KEY).unwrap();

    socket_mut.push(new_socket);

    BASE_SOCKET_FD + socket_mut.len() as c_int - 1  // len() is +1
}

#[no_mangle]
pub extern fn send(socket: c_int, msg: *const c_void, size: usize, _flags: c_int) -> ssize_t {
    let buf = unsafe { from_raw_parts(msg as *const u8, size) };

    // TODO: Get rid of all those unwraps maybe? Maybe try to recover?
    let mut socket_mut = SOCKET.lock().unwrap();
    let sock = socket_mut.get_mut((socket - BASE_SOCKET_FD) as usize).unwrap();
    sock.write(buf).unwrap() as isize
}

#[no_mangle]
pub extern fn recv(socket: c_int, msg: *mut c_void, size: usize, _flags: c_int) -> ssize_t {
    let buf = unsafe { from_raw_parts_mut(msg as *mut u8, size) };

    let mut socket_mut = SOCKET.lock().unwrap();
    let sock = socket_mut.get_mut((socket - BASE_SOCKET_FD) as usize).unwrap();
    sock.read(buf).unwrap() as isize
}

#[no_mangle]
pub extern fn listen(_socket: c_int, _backlog: c_int) -> c_int {
//    let socket_imm = SOCKET.lock().unwrap();
//    if BASE_ACCEPT_FD + socket_imm.len() as c_int >= BASE_SOCKET_FD {
//        return -1 // Maybe we should cycle? -1 is erroneous state and programs know it
//    }
//    drop(socket_imm);

    println!("yoooo1");
    let new_accept = LISTENER::listen(ADDRESS).unwrap();

    let mut accept_mut = ACCEPT.lock().unwrap();
    accept_mut.push(new_accept);

    println!("New listener: {}", BASE_ACCEPT_FD + accept_mut.len() as c_int - 1);

    BASE_ACCEPT_FD + accept_mut.len() as c_int - 1  // len() is +1
}

#[no_mangle]
pub extern fn accept(socket: c_int, _sockaddr: *const c_void, _address_len: *mut c_void) -> c_int {
    let mut socket_mut = SOCKET.lock().unwrap();
    if BASE_SOCKET_FD + socket_mut.len() as c_int >= BASE_ACCEPT_FD {
        return -1 // Maybe we should cycle? -1 is erroneous state and programs know it
    }

    let accept_imm = ACCEPT.lock().unwrap();
    let accepted = accept_imm.get((socket - BASE_ACCEPT_FD) as usize).unwrap().accept().unwrap().0;
    // TODO: Here we assume that listen is server and connect is client - not true. Must be configurable
    let new_socket = Connection::new(0, Box::new(accepted), true, SEED_KEY, REMOTE_KEY).unwrap();

    socket_mut.push(new_socket);

    BASE_SOCKET_FD + socket_mut.len() as c_int - 1  // len() is +1
}

//#[no_mangle]
//pub extern fn bind(_socket: c_int, _sockaddr: *const c_void, _address_len: *mut c_void) -> c_int {
//0
//}
//

//#[no_mangle]
//pub unsafe extern fn flush(s: &mut connection::Connection) { CONN.lock().unwrap().flush().unwrap() }
