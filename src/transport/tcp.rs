use transport::{Listener, Result, ReadWrite, Connector};
use std::net::{TcpListener, TcpStream};

type Tcp = TcpListener;

impl Listener for Tcp {

    fn listen(addr: &'static str) -> Result<Self> {
        unimplemented!()
    }

    fn accept(&self) -> Result<Box<dyn ReadWrite>> {
        unimplemented!()
    }

//    fn incoming(&self) -> dyn Iterator<Item=i32> {
//        unimplemented!()
//    }
}

impl Connector for Tcp {
    fn connect(addr: &'static str) -> Result<Box<dyn ReadWrite>> {
        match TcpStream::connect(addr) {
            Ok(c) => Ok(Box::new(c)),
            Err(e) => Err(Box::new(e))
        }
    }
}
