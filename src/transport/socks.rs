use std::net::ToSocketAddrs;
use std::io::Result;

use super::{Connector, Listener, ReadWrite, Tcp};

pub struct Socks(Tcp);

pub fn handle_client(conn: &mut Box<dyn ReadWrite>) -> Result<()> {
    // I won't implement a whole SOCKS library in mage. That's not what mage does
    // This requires an external crate and I didn't find any good
    // Some "rusty way" code was removed at commit 4907e25294c5282c4e8341ee5d9ec0542fdc8d30
    // with structs and all

//    let mut buf2 = [0; 2];
//    let mut buf4 = [0; 4];

    // 0: version, 1: number of authentication schemes
//    conn.read_exact(&mut buf2)?;
    // 0: version, 1: command, 2: ???, 3: address type
//    conn.read_exact(&mut buf4)?;

    // Don't care about version/command/address type

    // 0: version, 1: code (0 = success), ???
//    let response = [5, 0, 0, 1, 127, 0, 0, 1, 0, 0];
//    conn.write_all(&response)?;

    Ok(())
}

pub fn handle_server(conn: &mut Box<dyn ReadWrite>) -> Result<()> {
    conn.write_all(&[5, 0])?;
    conn.write_all(&[5, 1, 0, 1])?;
    let mut buf = [0; 10];
    conn.read_exact(&mut buf)?;

    Ok(())
}

impl Listener for Socks {
    fn listen<A: ToSocketAddrs>(addr: A) -> Result<Self> {
        Ok(Self(Tcp::listen(addr)?))
    }

    fn accept(&self) -> Result<Box<dyn ReadWrite>> {
        let mut conn = self.0.accept()?;
        handle_client(&mut conn)?;

        Ok(conn)
    }
}

impl Connector for Socks {
    fn connect<A: ToSocketAddrs>(addr: A) -> Result<Box<dyn ReadWrite>> {
        // TODO: Socks proxy client
        let mut conn = Tcp::connect(addr)?;

        handle_server(&mut conn)?;

        Ok(conn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_transport;

    test_transport!(test_transport_socks, Socks);
}
