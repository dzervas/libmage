use std::sync::mpsc::{Sender, Receiver};

pub fn bridge(sender: Sender<Vec<u8>>, receiver: Receiver<Vec<u8>>, i: &'static str) {
    println!("[{}] Starting bridge loop {:?}, {:?}", i, sender, receiver);
    loop {
        let buf = receiver.recv().unwrap();
        if !buf.is_empty() {
            println!("Bridge[{}]: something moved! {:?}", i, buf.clone());
            sender.send(buf).unwrap();
        }
    }
}
