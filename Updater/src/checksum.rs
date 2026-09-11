use std::{
    fs::File,
    io::{self, Read},
    path::Path,
};

use sha2::{Digest, Sha256};

const BUFFER_SIZE: usize = 64 * 1024;

pub fn calculate_sha256(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;

    let mut hasher = Sha256::new();

    let mut buffer = [0_u8; BUFFER_SIZE];

    loop {
        let read = file.read(&mut buffer)?;

        if read == 0 {
            break;
        }

        hasher.update(&buffer[..read]);
    }

    let digest = hasher.finalize();

    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}
