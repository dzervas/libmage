extern crate base64;
extern crate rand;
extern crate structopt;

use std::convert::TryInto;
use std::fs::{read, write};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::thread::{sleep, spawn};
use std::time::Duration;

use mage::stream::{StreamIn, StreamOut};
use mage::tool::Address;
use mage::transport::*;

use structopt::StructOpt;
use rand::random;

#[derive(Debug, StructOpt)]
#[structopt(name = "mage")]
struct Opts {
    /// Set verbosity level (pass it up to 3 times)
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    verbose: i32,

    /// Give seed as a base64 string (NOT SAFE)
    #[structopt(short = "k", long = "key")]
    key: Option<String>,

    /// Input seed file
    #[structopt(short = "i", long = "input")]
    input: Option<PathBuf>,

    #[structopt(subcommand)]
    cmds: Command,
}

#[derive(Debug, StructOpt)]
enum Command {
    /// Handle mage keys
    Key {
        /// Generate Key
        #[structopt(short = "g", long = "gen")]
        gen: bool,

        /// ASCII armor output (base64)
        #[structopt(short = "a", long = "ascii")]
        armor: bool,

        /// Output to file
        #[structopt(short = "o", long = "output")]
        output: Option<PathBuf>,
    },

    /// Use mage as a network proxy
    Proxy {
        /// Mage address string (NOT the proxy side)
        /// Ex.: tcp+listen://my-server:4444 on one machine
        /// Ex.: tcp://my-server:4444 on the other
        #[structopt(short = "a", long = "address", default_value = "tcp://127.0.0.1:4444")]
        address: String,

        /// Proxy will listen for stream_channeleds
        #[structopt(short = "l", long = "listen")]
        proxy_listen: bool,

        /// Proxy port to use
        #[structopt(short = "p", long = "port")]
        proxy_port: u16,

        /// Address of the proxy to listen/connect to
        /// Ex.: -lp 4444 127.0.0.1 on machine with the browser
        /// Ex.: -p 4444 some-server.com on machine with access to the server
        proxy_addr: String,
    },
}

// #[no_mangle]
// pub fn main(_argc: i32, _argv: *const *const u8) {
fn main() {
    let opts: Opts = Opts::from_args();

    let mut key: Vec<u8> = if opts.input.is_some() {
        read(opts.input.unwrap()).unwrap().to_vec()
    } else if opts.key.is_some() {
        base64::decode(opts.key.unwrap().as_bytes()).unwrap()
    } else {
        vec![]
    };

    match opts.cmds {
        Command::Key { gen, armor, output } => {
            if gen && key.is_empty() {
                key = random::<[u8; 32]>().to_vec();
            } else if key.is_empty() {
                eprintln!("Either pass --gen to generate a key or give --input <key_file>");
                return;
            }

            if armor {
                key = base64::encode(key.as_slice()).into_bytes();
            }

            match output {
                Some(d) => write(&d, key).unwrap(),
                None => {
                    if armor {
                        let key_str = String::from_utf8(key).unwrap();
                        println!(
                            "Key:\t{}",
                            key_str
                        )
                    } else {
                        println!("Key:\t{:?}", key)
                    }
                }
            }
        }
        Command::Proxy {
            address,
            proxy_listen,
            proxy_port,
            proxy_addr,
        } => {
            let mage_addr = Address::parse(address);
            let host_port = format!("{}:{}", mage_addr.host, mage_addr.port);
            println!("Key: {:?}", key);
            let key_array: [u8; 32] = key.as_slice().try_into().unwrap();

            // TODO: This is temporary - select transport on runtime
            let conn = if mage_addr.listen {
                let listener = Tcp::listen(host_port).unwrap();
                //                println!("Listening for mage stream_channeled at {} over Tcp", host_port);
                listener.accept().unwrap()
            } else {
                //                println!("Mage connecting to {} over Tcp", host_port);
                Tcp::connect(host_port).unwrap()
            };

            println!("Mage stream_channeled opened! Spawning communication thread");
            // While it's wrong to assume that if we listen we're server,
            // it's safe to assume and it's just about the proxy tool
            let mut stream_in = StreamIn::new(conn.0, key_array);
            let mut stream_out = StreamOut::new(conn.1, key_array);
            println!("AAA");

            let mut proxy_conn = if proxy_listen {
                let listener = TcpListener::bind((proxy_addr.as_str(), proxy_port)).unwrap();
                listener.accept().unwrap().0
            } else {
                TcpStream::connect((proxy_addr.as_str(), proxy_port)).unwrap()
            };

            let mut proxy_conn2 = proxy_conn.try_clone().unwrap();

            let _thread_tx = spawn(move || {
                loop {
                    let mut buf = [0; 2048];
                    let length = proxy_conn.read(&mut buf).unwrap();

                    if length > 0 {
                        println!("sending {} bytes: ", length);
                        stream_out.chunk(1, 1, &buf[..length]).unwrap();
                    }
                }
            });

            let _thread_rx = spawn(move || {
                loop {
                    let packet = stream_in.dechunk().unwrap();

                    if !packet.data.is_empty() {
                        proxy_conn2.write_all(packet.data.as_slice()).unwrap();
                        proxy_conn2.flush().unwrap();
                    }

                    println!("Proxy: something moved!");
                }
            });

            println!("Starting mage loop");
            loop {
                sleep(Duration::from_secs(1));
            }
        }
    }
}
