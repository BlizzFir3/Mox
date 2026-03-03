use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use anyhow::{Result, Context};
use rusqlite::Connection;
use crate::core::hasher;

pub struct ModImporter<'a> {
	pub db_conn: &'a Connection,
	pub store_path: PathBuf,
}

impl<'a> ModImporter<'a> {
	pub fn new(conn: &'a Connection, store_path: PathBuf) -> Self {
		Self { db_conn: conn, store_path }
	}

	pub fn import_mod(&self, mod_name: &str, source_dir: &Path) -> Result<()> {
		self.db_conn.execute(
			"INSERT INTO mods (name, source_path) VALUES (?, ?)",
			[mod_name, source_dir.to_str().unwrap()],
		)?;
		let mod_id = self.db_conn.last_insert_rowid();

		for entry in WalkDir::new(source_dir).into_iter().filter_map(|e| e.ok()) {
			let path = entry.path();
			if path.is_file() {
				// Skip files already inside the .mox directory to avoid infinite loops
				if path.components().any(|c| c.as_os_str() == ".mox") {
					continue;
				}
				self.process_file(mod_id, source_dir, path)?;
			}
		}
		Ok(())
	}

	pub fn process_file(&self, mod_id: i64, root: &Path, file_path: &Path) -> Result<()> {
		// 1. Generate hash
		let hash = hasher::hash_file(file_path).context("Failed to hash file")?;
		let size = file_path.metadata()?.len();

		// 2. Relative path for the DB manifest
		let relative_path = file_path.strip_prefix(root)?
			.to_str()
			.context("Non-UTF8 path")?;

		// 3. Physical Storage (The "Blob" store)
		let blob_dest = self.store_path.join(&hash);

		if !blob_dest.exists() {
			// Ensure the blobs directory exists
			if let Some(parent) = blob_dest.parent() {
				fs::create_dir_all(parent)?;
			}
			// Copy the file to the store.
			// Note: We use copy instead of hard_link here to ensure
			// the 'original' in the store is a fresh entry.
			fs::copy(file_path, &blob_dest)
				.with_context(|| format!("Failed to copy {} to store", file_path.display()))?;
		}

		// 4. Register blob in DB
		self.db_conn.execute(
			"INSERT OR REPLACE INTO mod_files (mod_id, blob_hash, relative_path) VALUES (?, ?, ?)",
			rusqlite::params![mod_id, hash, relative_path],
		)?;

		// 5. Link the file to this mod's instance
		self.db_conn.execute(
			"INSERT INTO mod_files (mod_id, blob_hash, relative_path) VALUES (?, ?, ?)",
			rusqlite::params![mod_id, hash, relative_path],
		)?;

		Ok(())
	}
}