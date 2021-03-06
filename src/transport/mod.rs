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
enable_transport!(http, "trans_http");

// A trait for bidirectional communication
use std::io::{Read, Write};
use std::net::ToSocketAddrs;

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
    fn accept(&self) -> Result<(Box<dyn Read + Send + Sync>, Box<dyn Write + Send + Sync>)>;
    // TODO: Make the damn iterator work
    //    fn incoming(&self) -> dyn Iterator<Item=i32>;
}

pub trait Connector: Sized + Send {
    fn connect<A: ToSocketAddrs>(addr: A) -> Result<(Box<dyn Read + Send + Sync>, Box<dyn Write + Send + Sync>)>;
}

pub trait Transport: Listener + Connector {}
impl<T: Listener + Connector + Sized + Send> Transport for T {}

#[cfg(test)]
pub mod tests {
    use super::*;
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
    #[cfg(not(tarapaulin_include))]
    pub fn test_listen_conn_inner<T: Transport>(
        succeds: bool,
        address: &'static str,
        client_to_listener: Vec<u8>,
        mut listener_to_client: Vec<u8>,
    ) {
        let mut client_to_listener_clone = client_to_listener.clone();
        let listener_to_client_clone = listener_to_client.clone();

        let thread = spawn(move || {
            let listener = assert_cond!(succeds, T::listen(address));
            let (mut reader, mut writer) = listener.accept().unwrap();

            let buffer = &mut client_to_listener_clone;

            reader.read_exact(buffer).unwrap();
            writer.write_all(listener_to_client_clone.as_slice()).unwrap();
            writer.flush().unwrap();

            assert_eq!(buffer.to_vec(), client_to_listener_clone);
        });

        sleep(Duration::from_millis(100));
        let (mut reader, mut writer) = T::connect(address).unwrap();

        let buffer = listener_to_client.as_mut();

        writer.write_all(client_to_listener.as_slice()).unwrap();
        writer.flush().unwrap();
        reader.read_exact(buffer).unwrap();

        assert_eq!(buffer.to_vec(), listener_to_client);

        assert_eq!(thread.join().is_ok(), succeds);
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
                // test_listen_conn_inner::<$t>(
                //     true,
                //     concat!("127.0.0.1:", $port),
                //     vec![1; 10000],
                //     vec![4; 10000],
                // );
                // These block (duh...)
                //                test_listen_conn_inner::<$t>(true, ("127.0.0.1", $port), vec![], vec![4; 10]);
                //                test_listen_conn_inner::<$t>(true, ("127.0.0.1", $port), vec![100; 10000], vec![]);
            }
        };
    }
}
