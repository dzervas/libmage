type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

use connection::ReadWrite;

pub trait Listener {
    fn listen(addr: &'static str) -> Result<Self>;
    fn accept(&self) -> Result<dyn ReadWrite>;  // TODO: Add some kind of address struct?
}

pub trait Connector {
    fn connect(addr: &'static str) -> Result<dyn ReadWrite>;
}

pub trait ReadWrite: Read + Write + Sync + Send {}
impl<T: ?Sized + Read + Write + Sync + Send> ReadWrite for T {}
