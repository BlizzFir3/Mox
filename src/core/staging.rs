/// Stage a file: Hash it and ensure it exists in the 'blobs' table
pub fn stage_file(conn: &rusqlite::Connection, file_path: &std::path::Path) -> anyhow::Result<()> {
	// 1. Calculate the hash (using our BLAKE3 hasher)
	let hash = crate::core::hasher::hash_file(file_path)?;
	let size = file_path.metadata()?.len();

	// 2. Insert into the blobs table (Atomic operation)
	conn.execute(
		"INSERT OR IGNORE INTO blobs (hash, size_bytes) VALUES (?, ?)",
		rusqlite::params![hash, size as i64],
	)?;

	// 3. Mark as 'staged' in a new table (we'll need to add a staging table)
	// For now, we can just print the confirmation
	println!("Staged: {} [{}]", file_path.display(), &hash[..8]);

	Ok(())
}