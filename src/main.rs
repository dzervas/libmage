mod packet;
mod stream;

fn main() {
    let mut p = packet::Packet::new(1, 1234, 0, "hello".as_bytes().to_vec());
    p.has_id(true);

    let s = p.serialize();
    println!("{:?}", p);
    println!("{:?}", s);

    let d = packet::deserialize(&s[..]);
    println!("{:?}", d);

    let st = stream::Stream::new(10, 10);
    println!("{:?}", st);

    let cs = st.chunk(13, "hello world wow".as_bytes().to_vec());
    println!("{:?}", cs);
}
