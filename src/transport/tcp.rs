use super::{Listener, Result, ReadWrite, Connector};
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
            Ok(d) => {
                // TODO: Remove this?
                d.0.set_nodelay(true).expect("set_nodelay call failed");
                Ok(Box::new(d.0))
            },
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
            Ok(d) => {
                // TODO: Remove this?
                d.set_nodelay(true).expect("set_nodelay call failed");
                Ok(Box::new(d))
            }
            Err(e) => Err(Box::new(e))
        }
    }
}
