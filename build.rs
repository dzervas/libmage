extern crate cbindgen;

use std::env;
use cbindgen::Language::C;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = env::var("OUT_DIR").unwrap();

    let cgen = cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_language(C)
        .generate();

    if cgen.is_ok() {
        // For some reason when using tarpaulin, the following fails
        cgen.unwrap().write_to_file(format!("{}/../../../mage.h", out_dir));
    } else {
        eprintln!("Unable to generate cbindgen headers!");
    }
}