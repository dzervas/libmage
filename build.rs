extern crate base64;
extern crate cbindgen;
extern crate heck;
extern crate url;

use std::env;
use std::fs::File;

use cbindgen::Language::C;
use heck::CamelCase;
use std::io::Write;
use url::Url;

#[cfg(target_os = "windows")]
const SETTINGS_PATH: &str = "src\\settings.rs";

#[cfg(not(target_os = "windows"))]
const SETTINGS_PATH: &str = "src/settings.rs";

macro_rules! __env_default_inner {
    ($env: expr, $def: expr, $post: expr) => {
        match env::var($env) {
            Ok(d) => $post(d),
            Err(_) => {
                eprintln!(concat!($env, " is undeclared, using default seed"));
                $def
            }
        }
    };
}

macro_rules! env_default_b64 {
    ($env: expr, $def: expr) => {
        __env_default_inner!($env, $def, |x: String| { base64::decode(&x).unwrap() })
    };
}

macro_rules! env_default {
    ($env: expr, $def: expr) => {
        __env_default_inner!($env, $def, |x: String| { x })
    };
}

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = env::var("OUT_DIR").unwrap();

    write_settings();

    let cgen = cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_language(C)
        .generate();

    if cgen.is_ok() {
        cgen.unwrap()
            .write_to_file(format!("{}/../../../mage.h", out_dir));
    } else {
        eprintln!("Unable to generate cbindgen headers!");
    }
}

fn write_settings() {
    // Seed key "expands" into another key.
    // Remote key should be the "expanded" key of the OTHER participant.
    let seed = env_default_b64!("MAGE_SEED", vec![1; 32]);
    let key = env_default_b64!(
        "MAGE_KEY",
        vec![
            252, 59, 51, 147, 103, 165, 34, 93, 83, 169, 45, 56, 3, 35, 175, 208, 53, 215, 129,
            123, 109, 27, 228, 125, 148, 111, 107, 9, 169, 203, 220, 6
        ]
    );
    let address = env_default!("MAGE_ADDRESS", "tcp://127.0.0.1:4444".to_string());

    let url = Url::parse(address.as_str()).unwrap();
    let scheme_parts = url.scheme().split("+").collect::<Vec<&str>>();
    let transport = scheme_parts.get(0).unwrap().to_camel_case();
    let listen = match scheme_parts.get(1) {
        Some(d) => d == &"listen",
        None => false,
    };
    let host = url.host_str().unwrap();
    let port = url.port().unwrap();

    let mut f = File::create(SETTINGS_PATH).unwrap();
    f.write_all(
        format!(
            "use crate::transport::*;

pub type TRANSPORT = {};
pub const LISTEN: bool = {};
pub const ADDRESS: &str = \"{}:{}\";
pub const SEED: &[u8] = &{:?};
pub const REMOTE_KEY: &[u8] = &{:?};",
            transport, listen, host, port, seed, key
        )
        .as_bytes(),
    )
    .unwrap();

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/settings.rs");
}
