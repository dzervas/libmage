type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

use std::io::{Read, Write};

pub mod tcp;

pub trait ReadWrite: Read + Write + Sync + Send {}
impl<T: ?Sized + Read + Write + Sync + Send> ReadWrite for T {}

pub trait Transport: Listener + Connector {}
impl<T: Listener + Connector> Transport for T {}

pub trait Listener: Sized {
    fn listen(addr: &'static str) -> Result<Self>;
    // TODO: Add some kind of address struct?
    fn accept(&self) -> Result<Box<dyn ReadWrite>>;
//    fn incoming(&self) -> dyn Iterator<Item=i32>;
}

pub trait Connector {
    fn connect(addr: &'static str) -> Result<Box<dyn ReadWrite>>;
}
