use transport::{Listener, Result, ReadWrite, Connector};
use std::net::{TcpListener, TcpStream};

pub type Tcp = TcpListener;

impl Listener for Tcp {
    fn listen(addr: &'static str) -> Result<Self> {
        match TcpListener::bind(addr) {
            Ok(d) => Ok(d),
            Err(e) => Err(Box::new(e))
        }
    }

    fn accept(&self) -> Result<Box<dyn ReadWrite>> {
        match (self as &TcpListener).accept() {
            // TODO: Use the SocketAddr (d.1)
            Ok(d) => Ok(Box::new(d.0)),
            Err(e) => Err(Box::new(e))
        }
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
