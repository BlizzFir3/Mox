use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use anyhow::{Result, Context};
use rusqlite::Connection;
use crate::core::{hasher, ignore::MoxIgnore};

pub struct ModImporter<'a> {
	pub db_conn: &'a Connection,
	pub store_path: PathBuf, // This path can be on another Drive (ex: D:/MoxStorage)
}

impl<'a> ModImporter<'a> {
	pub fn new(conn: &'a Connection, store_path: PathBuf) -> Self {
		Self { db_conn: conn, store_path }
	}

	pub fn import_all(&self, source_dir: &Path) -> Result<()> {
		let ignore = MoxIgnore::load();

		// Clear the temporary table to purge files deleted from disk
		self.db_conn.execute("DELETE FROM mod_files", [])?;

		for entry in WalkDir::new(source_dir).into_iter().filter_map(|e| e.ok()) {
			let path = entry.path();

			// Skip directories and ignored files
			if path.is_dir() || ignore.is_ignored(path) {
				continue;
			}

			self.process_file(source_dir, path)?;
		}
		Ok(())
	}

	fn process_file(&self, root: &Path, file_path: &Path) -> Result<()> {
		// 1. Hash the file content
		let hash = hasher::hash_file(file_path).context("Failed to hash file")?;
		let size = file_path.metadata()?.len();

		// 2. Compute relative path (ex: "Overlays/Tmex.package")
		let relative_path = file_path.strip_prefix(root)?
			.to_str()
			.context("Non-UTF8 path")?;

		// 3. Physical Storage: Move to global store if not exists
		let blob_dest = self.store_path.join(&hash);
		if !blob_dest.exists() {
			if let Some(parent) = blob_dest.parent() {
				fs::create_dir_all(parent)?;
			}
			// Copy from Game Drive to Storage Drive
			fs::copy(file_path, &blob_dest)
				.with_context(|| format!("Failed to copy {} to store", file_path.display()))?;
		}

		// 4. Update Staging area in DB
		self.db_conn.execute(
			"INSERT OR IGNORE INTO blobs (hash, size_bytes) VALUES (?, ?)",
			rusqlite::params![hash, size as i64],
		)?;

		self.db_conn.execute(
			"INSERT OR REPLACE INTO mod_files (blob_hash, relative_path) VALUES (?, ?)",
			rusqlite::params![hash, relative_path],
		)?;

		Ok(())
	}
}