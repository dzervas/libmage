use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::io::{Read, Result, Write};

use super::{Connector, Listener};

pub struct Tcp(TcpListener);

impl Listener for Tcp {
    fn listen<A: ToSocketAddrs>(addr: A) -> Result<Self> {
        Ok(Self(TcpListener::bind(addr)?))
    }

    fn accept(&self) -> Result<(Box<dyn Read + Send + Sync>, Box<dyn Write + Send + Sync>)> {
        let socket = self.0.accept()?.0;
        let result = (
            Box::new(socket.try_clone()?) as Box<dyn Read + Send + Sync>,
            Box::new(socket) as Box<dyn Write + Send + Sync>
        );

        Ok(result)
    }
}

impl Connector for Tcp {
    fn connect<A: ToSocketAddrs>(addr: A) -> Result<(Box<dyn Read + Send + Sync>, Box<dyn Write + Send + Sync>)> {
        let socket = TcpStream::connect(addr)?;
        let result = (
            Box::new(socket.try_clone()?) as Box<dyn Read + Send + Sync>,
            Box::new(socket) as Box<dyn Write + Send + Sync>
        );

        Ok(result)
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
