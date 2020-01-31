use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::io::Result;

use super::{Connector, Listener, ReadWrite};

pub struct Tcp(TcpListener);

impl Listener for Tcp {
    fn listen<A: ToSocketAddrs>(addr: A) -> Result<Self> {
        Ok(Self(TcpListener::bind(addr)?))
    }

    fn accept(&self) -> Result<Box<dyn ReadWrite>> {
        Ok(Box::new(self.0.accept()?.0))
    }
}

impl Connector for Tcp {
    fn connect<A: ToSocketAddrs>(addr: A) -> Result<Box<dyn ReadWrite>> {
        Ok(Box::new(TcpStream::connect(addr)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // This happens due to #[macro_export]
    // Compiler said: a macro annotated with `#[macro_export]` will be exported
    // at the root of the crate instead of the module where it is defined
    use crate::test_transport;

    test_transport!(test_transport_tcp, Tcp, "13370");
}