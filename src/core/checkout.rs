use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::fs;
use std::path::Path;

pub fn restore_commit(conn: &Connection, commit_hash_prefix: &str) -> Result<()> {
	// 1. Find the full commit hash from the prefix (like git does)
	let mut stmt = conn.prepare(
		"SELECT id, hash FROM commits WHERE hash LIKE ? LIMIT 1"
	)?;
	let (commit_id, full_hash): (i64, String) = stmt.query_row(
		[format!("{}%", commit_hash_prefix)],
		|row| Ok((row.get(0)?, row.get(1)?))
	).context("Commit not found")?;

	println!("Checking out {}...", &full_hash[..7]);

	// 2. Fetch all files associated with this commit
	let mut stmt = conn.prepare(
		"SELECT relative_path, blob_hash FROM commit_contents WHERE commit_id = ?"
	)?;

	let files = stmt.query_map(params![commit_id], |row| {
		Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
	})?;

	// 3. Restore files using Hard Links
	for file_result in files {
		let (rel_path, blob_hash) = file_result?;
		let target_path = Path::new(&rel_path);
		let blob_path = Path::new(".mox/blobs").join(&blob_hash);

		// Ensure parent directories exist (e.g., Mods/SubFolder/...)
		if let Some(parent) = target_path.parent() {
			fs::create_dir_all(parent)?;
		}

		// Remove existing file if it exists to replace it
		if target_path.exists() {
			fs::remove_file(target_path)?;
		}

		// Create the Hard Link: points the game folder to our Store
		fs::hard_link(&blob_path, target_path)
			.with_context(|| format!("Failed to link {} -> {}", blob_hash, rel_path))?;
	}

	println!("Switched to commit {}", &full_hash[..7]);
	Ok(())
}