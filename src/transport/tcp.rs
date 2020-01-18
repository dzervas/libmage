use std::net::{TcpListener, TcpStream};
use std::io::Result;

use super::{Listener, Connector};

pub type Tcp = TcpListener;

impl Listener<TcpStream> for Tcp {
    fn listen(addr: &'static str) -> Result<Self> {
        TcpListener::bind(addr)
    }

    fn accept(&self) -> Result<TcpStream> {
        Ok((self as &TcpListener).accept()?.0)
    }

//    fn incoming(&self) -> dyn Iterator<Item=i32> {
//        unimplemented!()
//    }
}

impl Connector<TcpStream> for Tcp {
    fn connect(addr: &'static str) -> Result<TcpStream> {
        TcpStream::connect(addr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::test_transport;

    test_transport!(test_transport_tcp, Tcp, TcpStream);
}