use std::io::{Read, Write};
use std::os::raw::c_void;
use std::cell::RefCell;
use std::slice::{from_raw_parts, from_raw_parts_mut};
use std::thread_local;

use crate::connection::Connection;
use crate::transport::*;

#[cfg(feature = "channels")]
use crate::channel::Channel;

#[cfg(not(test))]
use crate::settings::*;
use std::ffi::{CStr, CString};

#[cfg(test)]
type TRANSPORT = Tcp;

#[cfg(test)]
const ADDRESS: &'static str = "127.0.0.1:4444";

#[cfg(test)]
macro_rules! const_test_connect {
    () => {
        const LISTEN: bool = false;
        const SEED: &[u8] = &[1; 32];
        const REMOTE_KEY: &[u8] = &[252, 59, 51, 147, 103, 165, 34, 93, 83, 169, 45, 56, 3, 35, 175, 208, 53, 215, 129, 123, 109, 27, 228, 125, 148, 111, 107, 9, 169, 203, 220, 6];
    };
}

#[cfg(test)]
macro_rules! const_test_listen {
    () => {
        const LISTEN: bool = true;
        const SEED: &[u8] = &[2; 32];
        const REMOTE_KEY: &[u8] = &[171, 47, 202, 50, 137, 131, 34, 194, 8, 251, 45, 171, 80, 72, 189, 67, 195, 85, 198, 67, 15, 88, 136, 151, 203, 87, 73, 97, 207, 169, 128, 111];
    };
}

// Known keys: vec![1; 32] -> public vec![171, 47, 202, 50, 137, 131, 34, 194, 8, 251, 45, 171, 80, 72, 189, 67, 195, 85, 198, 67, 15, 88, 136, 151, 203, 87, 73, 97, 207, 169, 128, 111]
// Known keys: vec![2; 32] -> public vec![252, 59, 51, 147, 103, 165, 34, 93, 83, 169, 45, 56, 3, 35, 175, 208, 53, 215, 129, 123, 109, 27, 228, 125, 148, 111, 107, 9, 169, 203, 220, 6]

thread_local! {
    static SOCKET: RefCell<Vec<Connection>> = RefCell::new(Vec::new());
    static ACCEPT: RefCell<Vec<TRANSPORT>> = RefCell::new(Vec::new());
    #[cfg(feature = "channels")]
    static CHANNEL: RefCell<Vec<Channel>> = RefCell::new(Vec::new());
}

// TODO: Handle all panics - not supported by FFI, undefined behaviour

#[no_mangle]
pub extern fn ffi_connect_str(addr: *const i8) -> usize {
    let addr_str = unsafe { CStr::from_ptr(addr)}.to_str().unwrap();

    let new_conn = TRANSPORT::connect(addr_str).unwrap();

    #[cfg(test)]
    const_test_connect!();

    let conn = Connection::new(0, Box::new(new_conn), LISTEN, SEED, REMOTE_KEY).unwrap();

    new_socket(conn)
}

#[no_mangle]
pub extern fn ffi_connect() -> usize {
    let c_str = CString::new(ADDRESS).unwrap();
    ffi_connect_str(c_str.as_ptr() as *const i8)
}

#[no_mangle]
pub extern fn ffi_listen_str(addr: *const i8) -> usize {
    let addr_str = unsafe { CStr::from_ptr(addr)}.to_str().unwrap();
    let new_accept = TRANSPORT::listen(addr_str).unwrap();

    ACCEPT.with(move |cell| {
        let mut a = cell.borrow_mut();

        a.push(new_accept);

        #[cfg(not(test))]
        println!("New listener: {}", a.len() - 1);

        a.len() - 1  // len() is +1
    })
}

#[no_mangle]
pub extern fn ffi_listen() -> usize {
    let c_str = CString::new(ADDRESS).unwrap();
    ffi_listen_str(c_str.as_ptr() as *const i8)
}

#[no_mangle]
pub extern fn ffi_accept(socket: usize) -> usize {
    let accepted = ACCEPT.with(|cell| {
        let a = cell.borrow_mut();

        a.get(socket as usize).unwrap().accept().unwrap()
    });

    #[cfg(test)]
    const_test_listen!();
    let conn = Connection::new(0, accepted, LISTEN, SEED, REMOTE_KEY).unwrap();

    new_socket(conn)
}

#[no_mangle]
pub extern fn ffi_send(socket: usize, msg: *const c_void, size: usize) -> usize {
    // TODO: Use snappy compress https://doc.rust-lang.org/nomicon/ffi.html#creating-a-safe-interface to ensure safety of given buffers
    // TODO: Handle nulls
    let buf = unsafe { from_raw_parts(msg as *const u8, size) };

    SOCKET.with(|cell| {
        let mut s = cell.borrow_mut();

        // TODO: Get rid of all those unwraps maybe? Maybe try to recover?
        let sock = s.get_mut(socket as usize).unwrap();
        sock.write(buf).unwrap()
    })
}

#[no_mangle]
pub extern fn ffi_recv(socket: usize, msg: *mut c_void, size: usize) -> usize {
    let buf = unsafe { from_raw_parts_mut(msg as *mut u8, size) };

    SOCKET.with(|cell| {
        let mut s = cell.borrow_mut();

        let sock = s.get_mut(socket).unwrap();
        sock.read(buf).unwrap()
    })
}

#[no_mangle]
#[cfg(feature = "channels")]
pub extern fn ffi_get_channel(socket: usize, channel: u8) -> usize {
    let chan = SOCKET.with(|cell| {
        let mut s = cell.borrow_mut();

        let sock = s.get_mut(socket).unwrap();
        sock.get_channel(channel)
    });

    CHANNEL.with(|cell| {
        let mut s = cell.borrow_mut();

        s.push(chan);

        s.len() - 1
    })
}

#[no_mangle]
#[cfg(feature = "channels")]
pub extern fn ffi_channel_loop(socket: usize) {
    SOCKET.with(|cell| {
        let mut s = cell.borrow_mut();

        let sock = s.get_mut(socket).unwrap();
        sock.channel_loop().unwrap();
    });
}

#[no_mangle]
#[cfg(feature = "channels")]
pub extern fn ffi_send_channel(channel: usize, msg: *mut c_void, size: usize) -> usize {
    let buf = unsafe { from_raw_parts_mut(msg as *mut u8, size) };

    CHANNEL.with(|cell| {
        let mut s = cell.borrow_mut();

        let chan = s.get_mut(channel).unwrap();
        chan.write(buf).unwrap()
    })
}

#[no_mangle]
#[cfg(feature = "channels")]
pub extern fn ffi_recv_channel(channel: usize, msg: *mut c_void, size: usize) -> usize {
    let buf = unsafe { from_raw_parts_mut(msg as *mut u8, size) };

    CHANNEL.with(|cell| {
        let mut s = cell.borrow_mut();

        let chan = s.get_mut(channel).unwrap();
        chan.read(buf).unwrap()
    })
}

fn new_socket(conn: Connection) -> usize {
    SOCKET.with(move |cell| {
        let mut s = cell.borrow_mut();

        s.push(conn);

        #[cfg(not(test))]
        println!("New socket: {}", s.len() - 1);

        s.len() - 1  // len() is +1
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::{spawn, sleep};
    use std::time::Duration;

    // The problem when doing 2 (or more) blocking stuff in the same process is that
    // one of them has the Mutex and the other can't lock it. In this case,
    // I try from a thread to accept() and from the main thread connect()
    // accept() should already be running - but that blocks, so the thread is
    // locked and connect() can't run till accept() is done (which needs a connect()) etc.
    // Chicken & egg :)
    #[test]
    fn test_listen_connect() {
        let thread = spawn(|| {
            test_listening()
        });

        sleep(Duration::from_millis(100));
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
