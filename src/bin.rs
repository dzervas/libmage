extern crate base64;
extern crate bufstream;
#[macro_use]
extern crate may;
extern crate structopt;

use std::path::PathBuf;

use mage::tool::{key, Address};
use mage::transport::*;
use mage::connection::Connection;

use structopt::StructOpt;
use std::io::BufRead;
use bufstream::BufStream;
use std::net::{TcpListener, TcpStream};
use std::io::Write;

#[derive(Debug, StructOpt)]
#[structopt(name = "mage")]
struct Opts {
    /// Set verbosity level (pass it up to 3 times)
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    verbose: i32,

    /// Give seed as a base64 string (NOT SAFE)
    #[structopt(short = "s", long = "seed")]
    seed: Option<String>,

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
        /// Generate Seed & Public key
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

        /// Remote part mage public key
        #[structopt(short = "k", long = "public-key")]
        public_key: String,

        /// Proxy will listen for connections
        #[structopt(short = "l", long = "listen")]
        proxy_listen: bool,

        /// Proxy port to use
        #[structopt(short = "p", long = "port")]
        proxy_port: u16,

        /// Address of the proxy to listen/connect to
        /// Ex.: -lp 4444 127.0.0.1 on machine with the browser
        /// Ex.: -p 4444 some-server.com on machine with access to the server
        proxy_addr: String,
    }
}

fn main() {
    let opts: Opts = Opts::from_args();

    let mut seed = if opts.input.is_some() {
        key::seed_from_file(opts.input.unwrap()).unwrap()
    } else if opts.seed.is_some() {
        base64::decode(opts.seed.unwrap().as_bytes()).unwrap()
    } else {
        vec![]
    };

    match opts.cmds {
        Command::Key { gen, armor, output } => {

            if gen && seed.len() == 0 {
                seed = key::generate_seed();
            } else if seed.len() == 0 {
                eprintln!("Either pass --gen to generate seed or give --input <seed_file>");
                return;
            }

            let mut public_key = key::get_public_key(seed.clone());

            if armor {
                seed = base64::encode(seed.as_slice()).into_bytes();
                public_key = base64::encode(public_key.as_slice()).into_bytes();
            }

            match output {
                Some(d) => key::write_to_file(seed, public_key, d).unwrap(),
                None => {
                    if armor {
                        let seed_str = String::from_utf8(seed).unwrap();
                        let public_key_str = String::from_utf8(public_key).unwrap();
                        println!("Keypair Seed:\t{}\nPublic Key:\t{}", seed_str, public_key_str)
                    } else { println!("Keypair Seed:\t{:?}\nPublic Key:\t{:?}", seed, public_key) }
                }
            }
        }
        Command::Proxy { address, public_key, proxy_listen, proxy_port, proxy_addr } => {
            let mage_addr = Address::parse(address);
            let host_port = format!("{}:{}", mage_addr.host, mage_addr.port);

            let remote_key = base64::decode(public_key.as_bytes()).unwrap();

            // TODO: This is temporary - select transport on runtime
            let conn = if mage_addr.listen {
                let listener = Tcp::listen(host_port).unwrap();
//                println!("Listening for mage connection at {} over Tcp", host_port);
                listener.accept().unwrap()
            } else {
//                println!("Mage connecting to {} over Tcp", host_port);
                Tcp::connect(host_port).unwrap()
            };

            println!("Mage connection opened! Spawning communication thread");
            // While it's wrong to assume that if we listen we're server,
            // it's safe to assume and it's just about the proxy tool
            let mut connection = Box::new(Connection::new(0, Box::new(conn), mage_addr.listen, seed.as_slice(), remote_key.as_slice()).unwrap());
            println!("AAA");

            let proxy_conn = if proxy_listen {
                let listener = TcpListener::bind((proxy_addr.as_str(), proxy_port)).unwrap();
                listener.accept().unwrap().0
            } else {
                TcpStream::connect((proxy_addr.as_str(), proxy_port)).unwrap()
            };

            let mut proxy_buf = BufStream::new(proxy_conn.try_clone().unwrap());
            let mut proxy_buf2 = BufStream::new(proxy_conn);

            let ch = connection.get_channel(1);
            let (conn_tx, conn_rx) = (ch.sender, ch.receiver);

            go!(move || {
                loop {
                    println!("Read");
                    let buf = proxy_buf.fill_buf().unwrap();
                    let length = buf.len();
                    println!("Read2");
                    if length > 0 {
                        println!("sending {} bytes: {:?}", length, buf.clone());
                        conn_tx.send(buf.to_vec()).unwrap();
                    }
                    proxy_buf.consume(length);
                }
            });

            println!("here");

            go!(move || {
                loop {
                    println!("Recv");
                    let d = conn_rx.recv().unwrap();
                    println!("Recv2");
                    if !d.is_empty() {
                        proxy_buf2.write_all(d.as_slice()).unwrap();
                        proxy_buf2.flush().unwrap();
                    }
                    println!("Proxy: something moved!");
                }
            });

            println!("Starting mage loop");
            loop {
                connection.channel_loop().unwrap();
                println!("Mage: something moved!")
            }
        }
    }
}
