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

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "windows")]
    const TEST_FILE_PATH: &str = r".mage-test";

    #[cfg(not(target_os = "windows"))]
    const TEST_FILE_PATH: &str = r".mage-test";

    #[test]
    fn test_generation() {
        let seed = generate_seed();

        assert_eq!(seed.len(), kx::SEEDBYTES);
        // Maybe test entropy? But this starts testing sodiumoxide, not mage...
    }

    #[test]
    fn test_get_public_key() {
        let pk_one = get_public_key(vec![1;32]);
        let pk_two = get_public_key(vec![2;32]);

        assert_eq!(pk_one.len(), kx::PUBLICKEYBYTES);
        assert_eq!(pk_two.len(), kx::PUBLICKEYBYTES);

        assert_eq!(pk_one, vec![171, 47, 202, 50, 137, 131, 34, 194, 8, 251, 45, 171, 80, 72, 189, 67, 195, 85, 198, 67, 15, 88, 136, 151, 203, 87, 73, 97, 207, 169, 128, 111]);
        assert_eq!(pk_two, vec![252, 59, 51, 147, 103, 165, 34, 93, 83, 169, 45, 56, 3, 35, 175, 208, 53, 215, 129, 123, 109, 27, 228, 125, 148, 111, 107, 9, 169, 203, 220, 6]);
    }

    #[test]
    fn test_files() {
        let public_key = get_public_key(vec![1; 32]);

        write_to_file(vec![1; 32], public_key.clone(), PathBuf::from(TEST_FILE_PATH)).unwrap();

        let seed_file = seed_from_file(PathBuf::from(TEST_FILE_PATH)).unwrap();
        let public_key_file = get_public_key(seed_file.clone());

        assert_eq!(seed_file, vec![1; 32]);
        assert_eq!(public_key, public_key_file);
    }
}
