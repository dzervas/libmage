use super::{Listener, Result, Connector};
use std::net::{TcpListener, TcpStream};

pub type Tcp = TcpListener;

impl Listener<TcpStream> for Tcp {
    fn listen(addr: &'static str) -> Result<Self> {
        match TcpListener::bind(addr) {
            Ok(d) => Ok(d),
            Err(e) => Err(Box::new(e))
        }
    }

    fn accept(&self) -> Result<TcpStream> {
        match (self as &TcpListener).accept() {
            Ok(d) => Ok(d.0),
            Err(e) => Err(Box::new(e))
        }
    }

//    fn incoming(&self) -> dyn Iterator<Item=i32> {
//        unimplemented!()
//    }
}

impl Connector<TcpStream> for Tcp {
    fn connect(addr: &'static str) -> Result<TcpStream> {
        match TcpStream::connect(addr) {
            Ok(d) => Ok(d),
            Err(e) => Err(Box::new(e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::test_transport;

    test_transport!(test_transport_tcp, Tcp, TcpStream);
}