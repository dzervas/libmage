extern crate micro_http;

use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::io::{Read, Result, Write, Error, ErrorKind};

use super::super::error_str;
use super::{Connector, Listener};

// Client
use micro_http::{Request, Message, Method};
// Server
use micro_http::{Response, Version, StatusCode};

pub struct Http(TcpListener);

impl Listener for Http {
    fn listen<A: ToSocketAddrs>(addr: A) -> Result<Self> {
        Ok(Self(TcpListener::bind(addr)?))
    }

    fn accept(&self) -> Result<(Box<dyn Read + Send + Sync>, Box<dyn Write + Send + Sync>)> {
        let tcp_socket = self.0.accept()?.0;
        let socket_clone = HttpServer(tcp_socket.try_clone()?);
        let socket = HttpServer(tcp_socket);

        let result = (
            Box::new(socket_clone) as Box<dyn Read + Send + Sync>,
            Box::new(socket) as Box<dyn Write + Send + Sync>
        );

        Ok(result)
    }
}

pub struct HttpServer(TcpStream);

impl Read for HttpServer {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut original = [0; 2048];
        let length = self.0.read(&mut original)?;
        println!("{:?}", std::str::from_utf8(&original[..length]));

        let mut request = match Request::try_from(&original[..length]) {
            Ok(d) => d,
            Err(e) => return Err(error_str!("{:?}", e))
        };
        println!("{:?}", request);

        let body = request.body().expect("Request has no body");
        buf.copy_from_slice(body);

        Ok(body.len())
    }
}

impl Write for HttpServer {
    // I think that it's super weird that the server spontaneously
    // sends a respond. Probably should buffer them until a request
    // arrives - I don't know how this seems by opsec side
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let mut response = Response::new(Version::Http11, StatusCode::InternalServerError);
        response.with_body(&buf);

        response.send(&mut self.0)?;

        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<()> {
        self.0.flush()
    }
}

impl Connector for Http {
    fn connect<A: ToSocketAddrs>(addr: A) -> Result<(Box<dyn Read + Send + Sync>, Box<dyn Write + Send + Sync>)> {
        let tcp_socket = TcpStream::connect(addr)?;
        let socket_clone = HttpClient(tcp_socket.try_clone()?);
        let socket = HttpClient(tcp_socket);

        let result = (
            Box::new(socket_clone) as Box<dyn Read + Send + Sync>,
            Box::new(socket) as Box<dyn Write + Send + Sync>
        );

        Ok(result)
    }
}

pub struct HttpClient(TcpStream);

impl Read for HttpClient {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut response = match Response::receive(&mut self.0) {
            Ok(d) => d,
            Err(e) => return Err(error_str!(format!("Unable to receive response: {:?}", e)))
        };
        let body = response.body().expect("Empty body received!");
        buf.copy_from_slice(body);

        Ok(body.len())
    }
}

impl Write for HttpClient {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let mut request = Request::new(Method::Put, "/mage".to_string(), Version::Http11);
        request.with_body(buf);

        request.send(&mut self.0)?;

        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<()> {
        self.0.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // This happens due to #[macro_export]
    // Compiler said: a macro annotated with `#[macro_export]` will be exported
    // at the root of the crate instead of the module where it is defined
    use crate::test_transport;

    test_transport!(test_transport_http, Http, "13372");
}
