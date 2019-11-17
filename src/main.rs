extern crate clap;
extern crate sodiumoxide;

mod packet;
mod stream;
mod connection;
mod channel;

use packet::Packet;
use stream::Stream;

use std::net::{TcpStream, TcpListener};
use connection::Connection;
use std::io::{Read, Write};

use clap::{App, Arg};

fn main() {
    let mut p = Packet::new(1, 1234, 0, "hello".as_bytes().to_vec()).unwrap();
    p.has_id(true);

    let s = p.serialize().unwrap();
    println!("{:?}", p);
    println!("{:?}", s);

    let d = Packet::deserialize(&s[..]);
    println!("{:?}", d);

    let mut st = Stream::new(true, vec![1; 32].as_slice(), vec![252, 59, 51, 147, 103, 165, 34, 93, 83, 169, 45, 56, 3, 35, 175, 208, 53, 215, 129, 123, 109, 27, 228, 125, 148, 111, 107, 9, 169, 203, 220, 6].as_slice()).unwrap();
//    println!("{:?}", st);

    let cs = st.chunk(13, 14, "hello world wow".as_bytes().to_vec()).unwrap();
    println!("{:?}", cs);
    let cipher = cs.get(0).unwrap();

    let mut client = Stream::new(false, vec![2; 32].as_slice(), vec![171, 47, 202, 50, 137, 131, 34, 194, 8, 251, 45, 171, 80, 72, 189, 67, 195, 85, 198, 67, 15, 88, 136, 151, 203, 87, 73, 97, 207, 169, 128, 111].as_slice()).unwrap();
    let ds = client.dechunk(cipher.to_vec());
    println!("{:?}", ds);
    println!("----------------------------------------------------------------------------------");

    let opts = App::new("mage")
        .version("1.0")
        .about("Testing suite for mage protocol")
        .author("Dimitris Zervas")
        .arg(Arg::with_name("listen")
            .short("l")
            .long("listen")
            .help("Listen for connection @ 127.0.0.1:4444"))
        .get_matches();

    let tcp: TcpStream;
    if opts.is_present("listen") {
        let listen = TcpListener::bind("localhost:4444").unwrap();
        tcp = listen.accept().unwrap().0;
    } else {
        tcp = TcpStream::connect("localhost:4444").unwrap();
    }
    let mut writer = tcp.try_clone().unwrap();
    let mut reader = tcp.try_clone().unwrap();
    let mut conn = Connection::new(&mut reader, &mut writer, true, vec![1; 32].as_slice(), vec![2; 32].as_slice()).unwrap();

    println!("Write: {:?}", conn.write("lala".as_bytes()));
    let mut buf = [0u8; 32];
    println!("Read: {:?}", conn.read(&mut buf));

    println!("~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~");
    println!("{:?}", buf);
}
