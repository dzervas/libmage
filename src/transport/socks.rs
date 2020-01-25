use std::net::ToSocketAddrs;
use std::io::{Error, ErrorKind, Result};

use super::{Connector, Listener, ReadWrite, Tcp};
use crate::error_str;

pub struct Socks(Tcp);

impl Socks {
    fn handle_client(&self, conn: &mut Box<dyn ReadWrite>) -> Result<()> {
        let mut hbuffer = [0; 2];
        let mut rbuffer = [0; 4];

        conn.read_exact(&mut hbuffer);
        let header: Header = hbuffer.into();
        conn.read_exact(&mut rbuffer);
        let request: Request = rbuffer.into();

        if header.version != 5 || request.version != 5 {
            return Err(error_str!("Unsupported SOCKS version requested"))
        }

        // Probably shouldn't do something
        if request.command != Command::Connect {
            return Err(error_str!("Unsupported SOCKS command requested"));
        }

        let response: [u8; 10] = Response { version: 5, code: Code::Success }.into();
        conn.write(&response);

        Ok(())
    }
}

impl Listener for Socks {
    fn listen<A: ToSocketAddrs>(addr: A) -> Result<Self> {
        Ok(Self(Tcp::listen(addr)?))
    }

    fn accept(&self) -> Result<Box<dyn ReadWrite>> {
        let mut conn = self.0.accept()?;
        self.handle_client(&mut conn)?;

        Ok(conn)
    }
}

impl Connector for Socks {
    fn connect<A: ToSocketAddrs>(addr: A) -> Result<Box<dyn ReadWrite>> {
        // TODO: Socks proxy client
        Ok(Tcp::connect(addr)?)
    }
}

struct Header {
    version: u8,
    authn: u8,  // Unused
}

impl From<[u8; 2]> for Header {
    fn from(data: [u8; 2]) -> Self {
        Self { version: data[0], authn: data[1] }
    }
}

struct Request {
    version: u8,
    command: Command,
    // Another field is missing! Dunno why!
    addrtype: u8,
}

impl From<[u8; 4]> for Request {
    fn from(data: [u8; 4]) -> Self {
        Self { version: data[0], command: Command::from(data[1]), addrtype: data[3] }
    }
}

struct Response {
    version: u8,
    code: Code,
    // Address fields missing!
}

impl From<Response> for [u8; 10] {
    fn from(data: Response) -> Self {
        [data.version, data.code as u8, 0, 1, 127, 0, 0, 1, 0, 0]
    }
}

#[repr(u8)]
enum Code {
    Success = 0,
    Failure = 1,
    RuleFailure = 2,
    NetworkUnreachable = 3,
    HostUnreachable = 4,
    ConnectionRefused = 5,
    TtlExpired = 6,
    CommandNotSupported = 7,
    AddrTypeNotSupported = 8,
}

#[repr(u8)]
#[derive(PartialEq)]
enum Command {
    Unknown = 0,
    Connect = 1,
    Bind = 2,
    UdpAssociate = 3,
}

impl From<u8> for Command {
    fn from(n: u8) -> Self {
        match n {
            1 => Self::Connect,
            2 => Self::Bind,
            3 => Self::UdpAssociate,
            _ => Self::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_transport;

    test_transport!(test_transport_socks, Socks);
}
