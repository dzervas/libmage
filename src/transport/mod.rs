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

//pub trait Transport: Listener + Connector {}
//impl<T: Listener + Connector> Transport for T {}


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
