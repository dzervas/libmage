extern crate bincode;

mod packet;

//use bincode::{deserialize, serialize};

fn main() {
//    let s = String::from("hello").into_bytes();
    let mut p = packet::Packet::new(1, "hello".as_bytes());
    println!("{:?}", p);

    let s = p.serialize();
    println!("{:?}", s);

    let d: packet::Packet = packet::deserialize(&s[..]);
    println!("{:?}", d);
}
