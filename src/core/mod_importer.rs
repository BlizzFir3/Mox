use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use anyhow::{Result, Context};
use rusqlite::Connection;
use crate::core::hasher;

pub struct ModImporter<'a> {
	db_conn: &'a Connection,
	store_path: PathBuf,
}

impl<'a> ModImporter<'a> {
	pub fn new(conn: &'a Connection, store_path: PathBuf) -> Self {
		Self { db_conn: conn, store_path }
	}

	/// Scans a directory and indexes it into the Mox system
	pub fn import_mod(&self, mod_name: &str, source_dir: &Path) -> Result<()> {
		// 1. Create the mod entry in the database
		self.db_conn.execute(
			"INSERT INTO mods (name, source_path) VALUES (?, ?)",
			[mod_name, source_dir.to_str().unwrap()],
		)?;
		let mod_id = self.db_conn.last_insert_rowid();

		// 2. Iterate through files recursively
		for entry in WalkDir::new(source_dir).into_iter().filter_map(|e| e.ok()) {
			let path = entry.path();
			if path.is_file() {
				self.process_file(mod_id, source_dir, path)?;
			}
		}
		Ok(())
	}

	fn process_file(&self, mod_id: i64, root: &Path, file_path: &Path) -> Result<()> {
		// Generate hash for the file
		let hash = hasher::hash_file(file_path).context("Failed to hash file")?;
		let size = file_path.metadata()?.len();

		// Calculate relative path for the manifest (e.g., "textures/sky.dds")
		let relative_path = file_path.strip_prefix(root)?
			.to_str()
			.context("Non-UTF8 path")?;

		// 3. Register blob if it doesn't exist (deduplication)
		self.db_conn.execute(
			"INSERT OR IGNORE INTO blobs (hash, size_bytes) VALUES (?, ?)",
			rusqlite::params![hash, size as i64],
		)?;

		// 4. Link the file to the mod
		self.db_conn.execute(
			"INSERT INTO mod_files (mod_id, blob_hash, relative_path) VALUES (?, ?, ?)",
			rusqlite::params![mod_id, hash, relative_path],
		)?;

		// 5. TODO: Copy/Move file to .mox/blobs/ storage
		Ok(())
	}
}