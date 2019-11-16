mod packet;
mod stream;
mod connection;
mod channel;

use packet::Packet;
use stream::Stream;
use std::net::TcpStream;
use connection::Connection;
use std::io::{Read, Write};

fn main() {
    let mut p = Packet::new(1, 1234, 0, "hello".as_bytes().to_vec());
    p.has_id(true);

    let s = p.serialize();
    println!("{:?}", p);
    println!("{:?}", s);

    let d = Packet::deserialize(&s[..]);
    println!("{:?}", d);

    let st = Stream::new(10, true, true, true);
    println!("{:?}", st);

    let cs = st.chunk(13, 200, "hello world wow".as_bytes().to_vec());
    println!("{:?}", cs);

    let buf: Vec<u8> = vec![13, 129, 10, 104, 101, 108, 108, 111, 32, 119, 13, 1, 10, 111, 114, 108, 100, 32, 119, 111, 13, 65, 10, 119];
    let ds = st.dechunk(buf);
    println!("{:?}", ds);

    let tcp = TcpStream::connect("localhost:4444").unwrap();
    let writer = &mut tcp.try_clone().unwrap();
    let reader = &mut tcp.try_clone().unwrap();
    let mut conn = Connection::new(reader, writer, 10, true, true, true);

    conn.write("hello world".as_bytes());
    let mut buf = [0u8; 32];
    conn.read(&mut buf);
    println!("{:?}", buf);
}
