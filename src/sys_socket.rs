use std::io::{Read, Write};
use std::os::raw::{c_void, c_int};
use connection::Connection;
use std::slice::{from_raw_parts, from_raw_parts_mut};
use lazy_static::lazy_static;
use std::sync::Mutex;

//use transport;
use transport::{Connector, Listener, tcp};

// TODO: These should be cfg-ish. Friendlier config.
const ADDRESS: &'static str = "127.0.0.1:4444";
type CONNECTOR = tcp::Tcp;
type LISTENER = tcp::Tcp;

// Known keys: vec![1; 32] -> public vec![171, 47, 202, 50, 137, 131, 34, 194, 8, 251, 45, 171, 80, 72, 189, 67, 195, 85, 198, 67, 15, 88, 136, 151, 203, 87, 73, 97, 207, 169, 128, 111]
// Known keys: vec![2; 32] -> public vec![252, 59, 51, 147, 103, 165, 34, 93, 83, 169, 45, 56, 3, 35, 175, 208, 53, 215, 129, 123, 109, 27, 228, 125, 148, 111, 107, 9, 169, 203, 220, 6]

// Seed key "expands" into another key.
// Remote key should be the "expanded" key of the OTHER participant.
// Check bellow that when [1; 32] seed is used, the [252...] remote is used
const CONNECTOR_SEED_KEY: &[u8] = &[1; 32];
const CONNECTOR_REMOTE_KEY: &[u8] = &[252, 59, 51, 147, 103, 165, 34, 93, 83, 169, 45, 56, 3, 35, 175, 208, 53, 215, 129, 123, 109, 27, 228, 125, 148, 111, 107, 9, 169, 203, 220, 6];
const LISTENER_SEED_KEY: &[u8] = &[2; 32];
const LISTENER_REMOTE_KEY: &[u8] = &[171, 47, 202, 50, 137, 131, 34, 194, 8, 251, 45, 171, 80, 72, 189, 67, 195, 85, 198, 67, 15, 88, 136, 151, 203, 87, 73, 97, 207, 169, 128, 111];

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

// TODO: Handle all panics - not supported by FFI, undefined behaviour

#[no_mangle]
pub extern fn abi_connect(_socket: c_int, _sockaddr: *const c_void, _address_len: *mut c_void) -> c_int {
    eprintln!("Connecting");
    let new_conn = CONNECTOR::connect(ADDRESS).unwrap();

    let mut socket_mut = SOCKET.lock().unwrap();
    if BASE_SOCKET_FD + socket_mut.len() as c_int >= BASE_ACCEPT_FD {
        return -1 // Maybe we should cycle? -1 is erroneous state and programs know it
    }
    let new_socket = Connection::new(0, Box::new(new_conn), false, CONNECTOR_SEED_KEY, CONNECTOR_REMOTE_KEY).unwrap();

    socket_mut.push(new_socket);

    BASE_SOCKET_FD + socket_mut.len() as c_int - 1  // len() is +1
}

#[no_mangle]
pub extern fn abi_send(socket: c_int, msg: *const c_void, size: usize, _flags: c_int) -> isize {
    // TODO: Use snappy compress https://doc.rust-lang.org/nomicon/ffi.html#creating-a-safe-interface to ensure safety of given buffers
    // TODO: Handle nulls
    let buf = unsafe { from_raw_parts(msg as *const u8, size) };
    println!("-> {:?}", buf);

    // TODO: Get rid of all those unwraps maybe? Maybe try to recover?
    let mut socket_mut = SOCKET.lock().unwrap();
    let sock = socket_mut.get_mut((socket - BASE_SOCKET_FD) as usize).unwrap();
    sock.write(buf).unwrap() as isize
}

#[no_mangle]
pub extern fn abi_recv(socket: c_int, msg: *mut c_void, size: usize, _flags: c_int) -> isize {
    let buf = unsafe { from_raw_parts_mut(msg as *mut u8, size) };

    let mut socket_mut = SOCKET.lock().unwrap();
    let sock = socket_mut.get_mut((socket - BASE_SOCKET_FD) as usize).unwrap();
    sock.read(buf).unwrap() as isize
}

#[no_mangle]
pub extern fn abi_listen(_socket: c_int, _backlog: c_int) -> c_int {
    let new_accept = LISTENER::listen(ADDRESS).unwrap();

    let mut accept_mut = ACCEPT.lock().unwrap();
    accept_mut.push(new_accept);

    println!("New listener: {}", BASE_ACCEPT_FD + accept_mut.len() as c_int - 1);

    BASE_ACCEPT_FD + accept_mut.len() as c_int - 1  // len() is +1
}

#[no_mangle]
pub extern fn abi_accept(socket: c_int, _sockaddr: *const c_void, _address_len: *mut c_void) -> c_int {
    let accept_imm = ACCEPT.lock().unwrap();
    println!("Waiting...");
    let accepted = accept_imm.get((socket - BASE_ACCEPT_FD) as usize).unwrap().accept().unwrap().0;
    drop(accept_imm);
    println!("Unlocked");

    let mut socket_mut = SOCKET.lock().unwrap();
    if BASE_SOCKET_FD + socket_mut.len() as c_int >= BASE_ACCEPT_FD {
        return -1 // Maybe we should cycle? -1 is erroneous state and programs know it
    }

    // TODO: Here we assume that listen is server and connect is client - not true. Must be configurable
    let new_socket = Connection::new(0, Box::new(accepted), true, LISTENER_SEED_KEY, LISTENER_REMOTE_KEY).unwrap();

    socket_mut.push(new_socket);

    BASE_SOCKET_FD + socket_mut.len() as c_int - 1  // len() is +1
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr::{null, null_mut};
    use std::thread::{spawn, sleep};
    use std::time::Duration;

    // TODO: Make the mutex behave here!
    // The problem when doing 2 (or more) blocking stuff in the same process is that
    // one of them has the Mutex and the other can't lock it. In this case,
    // I try from a thread to accept() and from the main thread connect()
    // accept() should already be running - but that blocks, so the thread is
    // locked and connect() can't run till accept() is done (which needs a connect()) etc.
    // Chicken & egg :)
//    #[test]
    #[cfg_attr(tarpaulin, skip)]
    fn listening() {
        let thread = spawn(move || {
            let listener = abi_listen(0, 0);
            let sock = abi_accept(listener, null(), null_mut());

            let mut data = [4; 10];

            test_recv(sock, &mut data);
            test_send(sock, data.to_vec());

            assert_eq!(data, [1; 10]);
        });

        sleep(Duration::from_millis(100));
        let sock = abi_connect(0, null(), null_mut());

        let mut data = [1; 10];

        test_send(sock, data.to_vec());
        test_recv(sock, &mut data);

        assert_eq!(data, [4; 10]);

        thread.join().unwrap();
    }

    #[cfg_attr(tarpaulin, skip)]
    fn test_send(sock: c_int, data: Vec<u8>) -> isize {
        println!("Send");
        let med_buf = data.as_ptr();
        let buf = med_buf as *const _;

        abi_send(sock, buf, data.len(), 0)
    }

    #[cfg_attr(tarpaulin, skip)]
    fn test_recv(sock: c_int, data: &mut [u8]) -> isize {
        println!("Recv");
        let buf = data.as_mut_ptr() as *mut _;

        abi_recv(sock, buf, data.len(), 0)
    }
}
