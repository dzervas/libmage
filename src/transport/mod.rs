mod tcp;
pub use self::tcp::*;

// Easier result
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

// A trait for bidirectional communication
use std::io::{Read, Write};
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
pub trait Listener<T: ReadWrite>: Sized + Send {
    fn listen(addr: &'static str) -> Result<Self>;
    // TODO: Add some kind of address struct?
    fn accept(&self) -> Result<T>;
    // TODO: Make the damn iterator work
//    fn incoming(&self) -> dyn Iterator<Item=i32>;
}

pub trait Connector<T: ReadWrite>: Sized + Send {
    fn connect(addr: &'static str) -> Result<T>;
}

pub trait Transport<RW: ReadWrite>: Listener<RW> + Connector<RW> {}
impl<T: Listener<RW> + Connector<RW> + Sized + Send, RW: ReadWrite> Transport<RW> for T {}


// Buffered bidirectional communication
//pub struct BufReadWriter<T: ReadWrite> {
//    pub reader: BufReader<T>,
//    pub writer: BufWriter<T>,
//}
//
//impl BufReadWriter<dyn ReadWrite> {
//    // TO/DO: Is this Box required? Doesn't it just eat up memory?
//    pub fn new(mut rw: Box<dyn ReadWrite>) -> Self {
//        let raw_rw = rw.deref_mut();
//
//        Self {
//            reader: BufReader::new(Box::new(raw_rw)),
//            writer: BufWriter::new(Box::new(raw_rw))
//        }
//    }
//}
//
//impl Read for BufReadWriter<dyn ReadWrite> {
//    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
//        self.reader.read(buf)
//    }
//}
//
//impl Write for BufReadWriter<dyn ReadWrite> {
//    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
//        self.writer.write(buf)
//    }
//
//    fn flush(&mut self) -> std::io::Result<()> {
//        self.writer.flush()
//    }
//}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::{spawn, sleep};
    use std::time::Duration;
    use std::borrow::BorrowMut;

    #[macro_export]
    macro_rules! assert_cond {
        ($succ: expr, $func: expr) => {
            match $func {
                Ok(d) => d,
                Err(e) => return assert!($succ, e.to_string())
            };
        }
    }

    // Test listen, accept, connect
    #[cfg_attr(tarpaulin, skip)]
    pub fn test_listen_conn_inner<T: Transport<RW>, RW: ReadWrite>(succ: bool, addr: &'static str, c2l: Vec<u8>, mut l2c: Vec<u8>) {
        let mut c2l_clone = c2l.clone();
        let l2c_clone = l2c.clone();

        let thread = spawn(move || {
            let listener = assert_cond!(succ, T::listen(addr));
            let mut rw = listener.accept().unwrap();

            let buffer = c2l_clone.borrow_mut();

            rw.read(buffer).unwrap();
            rw.write(l2c_clone.as_slice()).unwrap();

            assert_eq!(buffer.to_vec(), c2l_clone);
        });

        sleep(Duration::from_millis(100));
        let mut rw = T::connect(addr).unwrap();

        let buffer = l2c.as_mut();

        rw.write(c2l.as_slice()).unwrap();
        rw.read(buffer).unwrap();

        assert_eq!(buffer.to_vec(), l2c);

        assert_eq!(thread.join().is_ok(), succ);
    }

    // Transport Tests
    #[macro_export]
    macro_rules! test_transport {
        ($name:ident, $t:ty, $rw:ty) => {
            use transport::tests::test_listen_conn_inner;

            #[test]
            fn $name() {
                test_listen_conn_inner::<$t, $rw>(true, "127.0.0.1:13337", vec![1; 10], vec![4; 10]);
                test_listen_conn_inner::<$t, $rw>(true, "127.0.0.1:13337", vec![1; 512], vec![4; 512]);
                test_listen_conn_inner::<$t, $rw>(true, "127.0.0.1:13337", vec![1; 10000], vec![4; 10000]);
                // These block (duh...)
//                test_listen_conn_inner::<$t, $rw>(true, "127.0.0.1:13337", vec![], vec![4; 10]);
//                test_listen_conn_inner::<$t, $rw>(true, "127.0.0.1:13337", vec![100; 10000], vec![]);
            }
        }
    }
}
