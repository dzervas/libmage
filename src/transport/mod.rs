use std::io::Result;

macro_rules! enable_transport {
    ($trans: ident, $feature: expr) => {
        #[cfg(feature = $feature)]
        mod $trans;
        #[cfg(feature = $feature)]
        pub use self::$trans::*;
    };
}

// Transport definition
enable_transport!(tcp, "trans_tcp");
enable_transport!(socks, "trans_socks");

// A trait for bidirectional communication
use std::io::{Read, Write};
use std::net::ToSocketAddrs;

pub trait ReadWrite: Read + Write + Sync + Send {}
impl<T: ?Sized + Read + Write + Sync + Send> ReadWrite for T {}

// Listener/Connector traits
// It should be noted that both can be part of either
// a server or a client! If you have a reverse shell,
// the listener is the server, but if you have a bind
// shell the connector is the server!
// Client: pwned machine
// Server: attacker's machine
// Who listens and who connects is irrelevant
pub trait Listener: Sized + Send {
    fn listen<A: ToSocketAddrs>(addr: A) -> Result<Self>;
    // TODO: Add some kind of address struct?
    fn accept(&self) -> Result<Box<dyn ReadWrite>>;
    // TODO: Make the damn iterator work
    //    fn incoming(&self) -> dyn Iterator<Item=i32>;
}

pub trait Connector: Sized + Send {
    fn connect<A: ToSocketAddrs>(addr: A) -> Result<Box<dyn ReadWrite>>;
}

pub trait Transport: Listener + Connector {}
impl<T: Listener + Connector + Sized + Send> Transport for T {}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::borrow::BorrowMut;
    use std::thread::{sleep, spawn};
    use std::time::Duration;

    #[macro_export]
    macro_rules! assert_cond {
        ($succ: expr, $func: expr) => {
            match $func {
                Ok(d) => d,
                Err(e) => return assert!($succ, e.to_string()),
            };
        };
    }

    // Test listen, accept, connect
    #[cfg_attr(tarpaulin, skip)]
    pub fn test_listen_conn_inner<T: Transport>(
        succ: bool,
        addr: &'static str,
        c2l: Vec<u8>,
        mut l2c: Vec<u8>,
    ) {
        let mut c2l_clone = c2l.clone();
        let l2c_clone = l2c.clone();

        let thread = spawn(move || {
            let listener = assert_cond!(succ, T::listen(addr));
            let mut rw = listener.accept().unwrap();

            let buffer = c2l_clone.borrow_mut();

            rw.read_exact(buffer).unwrap();
            rw.write_all(l2c_clone.as_slice()).unwrap();

            assert_eq!(buffer.to_vec(), c2l_clone);
        });

        sleep(Duration::from_millis(100));
        let mut rw = T::connect(addr).unwrap();

        let buffer = l2c.as_mut();

        rw.write_all(c2l.as_slice()).unwrap();
        rw.read_exact(buffer).unwrap();

        assert_eq!(buffer.to_vec(), l2c);

        assert_eq!(thread.join().is_ok(), succ);
    }

    // Transport Tests
    #[macro_export]
    macro_rules! test_transport {
        ($name:ident, $t:ty, $port:literal) => {
            use crate::transport::tests::test_listen_conn_inner;

            #[test]
            fn $name() {
                test_listen_conn_inner::<$t>(
                    true,
                    concat!("127.0.0.1:", $port),
                    vec![1; 10],
                    vec![4; 10],
                );
                test_listen_conn_inner::<$t>(
                    true,
                    concat!("127.0.0.1:", $port),
                    vec![1; 512],
                    vec![4; 512],
                );
                test_listen_conn_inner::<$t>(
                    true,
                    concat!("127.0.0.1:", $port),
                    vec![1; 10000],
                    vec![4; 10000],
                );
                // These block (duh...)
                //                test_listen_conn_inner::<$t>(true, ("127.0.0.1", $port), vec![], vec![4; 10]);
                //                test_listen_conn_inner::<$t>(true, ("127.0.0.1", $port), vec![100; 10000], vec![]);
            }
        };
    }
}
