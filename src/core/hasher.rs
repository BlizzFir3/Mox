use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use blake3::Hasher;

pub fn hash_file(path: &Path) -> io::Result<String> {
	let mut file = File::open(path)?;
	let mut hasher = Hasher::new();
	let mut buffer = [0; 8192]; // 8KB Buffer

	loop {
		let count = file.read(&mut buffer)?;
		if count == 0 { break; }
		hasher.update(&buffer[..count]);
	}

	Ok(hasher.finalize().to_string())
}