extern crate lazy_static;

use std::ffi::CStr;
use std::io::{Read, Write};
use std::os::raw::c_void;
use std::slice::{from_raw_parts, from_raw_parts_mut};
use std::sync::RwLock;

use lazy_static::lazy_static;

use crate::connection::Connection;
use crate::transport::*;

#[cfg(feature = "channels")]
use crate::channel::Channel;

#[cfg(not(test))]
use crate::settings::*;

#[cfg(test)]
type TRANSPORT = Tcp;

#[cfg(test)]
const ADDRESS: &str = "127.0.0.1:4444";

#[cfg(test)]
macro_rules! const_test_connect {
    () => {
        const LISTEN: bool = false;
        const SEED: &[u8] = &[1; 32];
        const REMOTE_KEY: &[u8] = &[
            252, 59, 51, 147, 103, 165, 34, 93, 83, 169, 45, 56, 3, 35, 175, 208, 53, 215, 129,
            123, 109, 27, 228, 125, 148, 111, 107, 9, 169, 203, 220, 6,
        ];
    };
}

#[cfg(test)]
macro_rules! const_test_listen {
    () => {
        const LISTEN: bool = true;
        const SEED: &[u8] = &[2; 32];
        const REMOTE_KEY: &[u8] = &[
            171, 47, 202, 50, 137, 131, 34, 194, 8, 251, 45, 171, 80, 72, 189, 67, 195, 85, 198,
            67, 15, 88, 136, 151, 203, 87, 73, 97, 207, 169, 128, 111,
        ];
    };
}

// Known keys: vec![1; 32] -> public vec![171, 47, 202, 50, 137, 131, 34, 194, 8, 251, 45, 171, 80, 72, 189, 67, 195, 85, 198, 67, 15, 88, 136, 151, 203, 87, 73, 97, 207, 169, 128, 111]
// Known keys: vec![2; 32] -> public vec![252, 59, 51, 147, 103, 165, 34, 93, 83, 169, 45, 56, 3, 35, 175, 208, 53, 215, 129, 123, 109, 27, 228, 125, 148, 111, 107, 9, 169, 203, 220, 6]

// Thread local is used instead of lazy_static, as the tests do not run with lazy_static.
// They deadlock. While that's test-specific problem, I can see it rising on apps as well
// The deadlock happens when you try to listen AND connect at the same time to yourself.
// As `connect` blocks till it connects and `accept` blocks till it accepts a connection
// However, thread local means that in go for example you can't:
// c := ffi_connect(...)
// ch := ffi_get_channel(...)
// go ffi_channel_loop()
// as `go <whatever>` is a different "thread" (for what Rust knows), so the `SOCKET` (created from
// the main thread) will not be there, `ffi_channel_loop` will die
// due to `SOCKET.get_mut(...).unwrap()`.
// (`go <whatever>` spawns a co-routine in go, check it out, it's amazing)
//
// Fuck.
// 2020 ~ The Witcher
lazy_static! {
    static ref SOCKET: RwLock<Vec<RwLock<Connection>>> = RwLock::new(Vec::new());
    static ref ACCEPT: RwLock<Vec<RwLock<TRANSPORT>>> = RwLock::new(Vec::new());
}

#[cfg(feature = "channels")]
lazy_static! {
    static ref CHANNEL: RwLock<Vec<RwLock<Channel>>> = RwLock::new(Vec::new());
}

// TODO: Handle all panics - not supported by FFI, undefined behaviour

// Helper functions
fn _connect(addr: &str, listen: bool, seed: &[u8], key: &[u8]) -> usize {
    let new_conn = TRANSPORT::connect(addr).unwrap();

    let conn = Connection::new(0, Box::new(new_conn), listen, seed, key).unwrap();

    new_socket(conn)
}

fn _listen(addr: &str) -> usize {
    let new_accept = TRANSPORT::listen(addr).unwrap();

    let mut accept_locked = ACCEPT.write().unwrap();

    accept_locked.push(RwLock::new(new_accept));

    #[cfg(not(test))]
    println!("New listener: {}", accept_locked.len() - 1);

    accept_locked.len() - 1 // len() is +1
}

fn _accept(socket: usize, listen: bool, seed: &[u8], key: &[u8]) -> usize {
    let accept_locked = ACCEPT.read().unwrap();
    let accepted = {
        // This unwraping is getting out of hand
        accept_locked
            .get(socket as usize)
            .unwrap()
            .read()
            .unwrap()
            .accept()
            .unwrap()
    };

    let conn = Connection::new(0, accepted, listen, seed, key).unwrap();

    new_socket(conn)
}

fn new_socket(conn: Connection) -> usize {
    let mut socket_locked = SOCKET.write().unwrap();

    socket_locked.push(RwLock::new(conn));

    #[cfg(not(test))]
    println!("New socket: {}", socket_locked.len() - 1);

    socket_locked.len() - 1 // len() is +1
}

// FFI API - Connection initialization
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn ffi_connect_opt(
    addr: *const i8,
    listen: u8,
    seed: *const u8,
    key: *const u8,
) -> usize {
    let addr_str = unsafe { CStr::from_ptr(addr) }.to_str().unwrap();
    let listen_bool = listen != 0;
    let seed_bytes = unsafe { from_raw_parts(seed, 32) };
    let key_bytes = unsafe { from_raw_parts(key, 32) };

    _connect(addr_str, listen_bool, seed_bytes, key_bytes)
}

#[no_mangle]
pub extern "C" fn ffi_connect() -> usize {
    #[cfg(test)]
    const_test_connect!();

    _connect(ADDRESS, LISTEN, SEED, REMOTE_KEY)
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn ffi_listen_opt(addr: *const i8) -> usize {
    let addr_str = unsafe { CStr::from_ptr(addr) }.to_str().unwrap();
    _listen(addr_str)
}

#[no_mangle]
pub extern "C" fn ffi_listen() -> usize {
    _listen(ADDRESS)
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn ffi_accept_opt(
    socket: usize,
    listen: u8,
    seed: *const u8,
    key: *const u8,
) -> usize {
    let listen_bool = listen != 0;
    let seed_bytes = unsafe { from_raw_parts(seed, 32) };
    let key_bytes = unsafe { from_raw_parts(key, 32) };

    _accept(socket, listen_bool, seed_bytes, key_bytes)
}

#[no_mangle]
pub extern "C" fn ffi_accept(socket: usize) -> usize {
    #[cfg(test)]
    const_test_listen!();

    _accept(socket, LISTEN, SEED, REMOTE_KEY)
}

// FFI API - Simple data transfer interface
#[no_mangle]
pub extern "C" fn ffi_send(socket: usize, msg: *const c_void, size: usize) -> usize {
    // TODO: Use snappy compress https://doc.rust-lang.org/nomicon/ffi.html#creating-a-safe-interface to ensure safety of given buffers
    // TODO: Handle nulls
    let buf = unsafe { from_raw_parts(msg as *const u8, size) };

    let socket_locked = SOCKET.read().unwrap();

    let mut sock = socket_locked.get(socket).unwrap().write().unwrap();
    sock.write(buf).unwrap()
}

#[no_mangle]
pub extern "C" fn ffi_recv(socket: usize, msg: *mut c_void, size: usize) -> usize {
    let buf = unsafe { from_raw_parts_mut(msg as *mut u8, size) };

    let socket_locked = SOCKET.read().unwrap();

    let mut sock = socket_locked.get(socket).unwrap().write().unwrap();
    sock.read(buf).unwrap()
}

// FFI API - Channel handling interface
#[no_mangle]
#[cfg(feature = "channels")]
pub extern "C" fn ffi_get_channel(socket: usize, channel: u8) -> usize {
    let chan = {
        let socket_locked = SOCKET.read().unwrap();

        let mut sock = socket_locked.get(socket).unwrap().write().unwrap();
        sock.get_channel(channel)
    };

    let mut channel_locked = CHANNEL.write().unwrap();

    channel_locked.push(RwLock::new(chan));

    channel_locked.len() - 1
}

#[no_mangle]
#[cfg(feature = "channels")]
pub extern "C" fn ffi_channel_loop(socket: usize) {
    let socket_locked = SOCKET.read().unwrap();

    let mut sock = socket_locked.get(socket).unwrap().write().unwrap();
    sock.channel_loop().unwrap();
}

// FFI API - Channel data transfer interface
#[no_mangle]
#[cfg(feature = "channels")]
pub extern "C" fn ffi_send_channel(channel: usize, msg: *mut c_void, size: usize) -> usize {
    let buf = unsafe { from_raw_parts_mut(msg as *mut u8, size) };

    let channel_locked = SOCKET.read().unwrap();
    let mut chan = channel_locked.get(channel).unwrap().write().unwrap();

    chan.write(buf).unwrap()
}

#[no_mangle]
#[cfg(feature = "channels")]
pub extern "C" fn ffi_recv_channel(channel: usize, msg: *mut c_void, size: usize) -> usize {
    let buf = unsafe { from_raw_parts_mut(msg as *mut u8, size) };

    let channel_locked = SOCKET.read().unwrap();
    let mut chan = channel_locked.get(channel).unwrap().write().unwrap();

    chan.read(buf).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::{sleep, spawn};
    use std::time::Duration;

    // The problem when doing 2 (or more) blocking stuff in the same process is that
    // one of them has the Mutex and the other can't lock it. In this case,
    // I try from a thread to accept() and from the main thread connect()
    // accept() should already be running - but that blocks, so the thread is
    // locked and connect() can't run till accept() is done (which needs a connect()) etc.
    // Chicken & egg :)
    // TODO: SOMETIMES this blocks
    #[test]
    fn test_listen_connect() {
        let thread = spawn(|| test_listening());

        sleep(Duration::from_millis(1000));
        test_connecting();
        assert!(thread.join().is_ok(), "Thread panicked!");
    }

    #[cfg_attr(tarpaulin, skip)]
    fn test_listening() {
        let listener = ffi_listen();

        let sock = ffi_accept(listener);

        let mut data = [4; 1000];

        test_recv(sock, &mut data);
        test_send(sock, data.to_vec());

        assert_eq!(data.to_vec(), vec![1; 1000]);
    }

    #[cfg_attr(tarpaulin, skip)]
    fn test_connecting() {
        let sock = ffi_connect();

        let mut data = [1; 1000];

        test_send(sock, data.to_vec());
        test_recv(sock, &mut data);

        assert_eq!(data.to_vec(), vec![1; 1000]);
    }

    #[cfg_attr(tarpaulin, skip)]
    fn test_send(sock: usize, data: Vec<u8>) -> usize {
        let med_buf = data.as_ptr();
        let buf = med_buf as *const _;

        ffi_send(sock, buf, data.len())
    }

    #[cfg_attr(tarpaulin, skip)]
    fn test_recv(sock: usize, data: &mut [u8]) -> usize {
        let buf = data.as_mut_ptr() as *mut _;

        ffi_recv(sock, buf, data.len())
    }
}
