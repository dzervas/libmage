use std::fs::{read, write};
use std::io::Result;
use std::path::PathBuf;

use rand::random;

pub fn generate_key() -> [u8; 8] {
    random();
}

pub fn key_from_file(path: PathBuf) -> Result<Vec<u8>> {
    read(path)
}

pub fn write_to_file(key: [u8; 8], mut path: PathBuf) -> Result<()> {
    write(&path, key)
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
        let seed = generate_key();

        assert_eq!(seed.len(), 8);
        // Maybe test entropy? But this starts testing Rust, not mage...
    }

    #[test]
    fn test_files() {
        write_to_file(
            vec![1; 32],
            public_key.clone(),
            PathBuf::from(TEST_FILE_PATH),
        )
        .unwrap();

        let seed_file = seed_from_file(PathBuf::from(TEST_FILE_PATH)).unwrap();
        let public_key_file = get_public_key(seed_file.clone());

        assert_eq!(seed_file, vec![1; 32]);
        assert_eq!(public_key, public_key_file);
    }
}
