extern crate base64;
extern crate structopt;

mod tool;
use tool::key;

use structopt::StructOpt;
use std::path::PathBuf;

#[derive(Debug, StructOpt)]
#[structopt(name = "mage")]
struct Opts {
    /// Set verbosity level (pass it up to 3 times)
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    verbose: i32,

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

        /// Input seed file
        #[structopt(short = "i", long = "input")]
        input: Option<PathBuf>,

        /// Output to file
        #[structopt(short = "o", long = "output")]
        output: Option<PathBuf>,
    },

    /// Use mage as a network proxy
    Proxy {
        /// Listen port for mage protocol
        #[structopt(short = "l", long = "listen")]
        listen: bool,
    }
}

fn main() {
    let opts: Opts = Opts::from_args();

    match opts.cmds {
        Command::Key { gen, armor, input, output } => {
            let seed: Vec<u8>;

            if gen {
                seed = key::generate_seed();
            } else if input.is_some() {
                seed = key::seed_from_file(input.unwrap()).unwrap();
            } else {
                eprintln!("Either pass --gen to generate seed or give --input <seed_file>");
                return;
            }

            let public_key = key::get_public_key(seed.clone());

//            if armor {
//                let seed_armor = base64::encode(seed.as_slice());
//                let public_key_armor = base64::encode(public_key.as_slice());
//            }

            match output {
                Some(d) => key::write_to_file(seed, public_key, d).unwrap(),
                None => {
                    println!("Seed: {:?}", seed);
                    println!("Public Key: {:?}", public_key);
                }
            }
        }
        Command::Proxy { listen: _listen } => {}
    }
}
