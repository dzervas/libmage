use std::fs::{read, write};
use std::io::Result;
use std::path::PathBuf;

use sodiumoxide::randombytes::randombytes;
use sodiumoxide::crypto::kx;

pub fn generate_seed() -> Vec<u8> {
    randombytes(kx::SEEDBYTES)
}

pub fn get_public_key(seed: Vec<u8>) -> Vec<u8> {
    let seed_struct = kx::Seed::from_slice(seed.as_slice()).expect("Seed struct could not be instantiated!");
    let public_key = kx::keypair_from_seed(&seed_struct).0;
    public_key.0.to_vec()
}

pub fn seed_from_file(path: PathBuf) -> Result<Vec<u8>> {
    read(path)
}

pub fn write_to_file(seed: Vec<u8>, public_key: Vec<u8>, mut path: PathBuf) -> Result<()> {
    write(&path, seed)?;
    path.set_extension("pub");
    write(&path, public_key)?;
    Ok(())
}