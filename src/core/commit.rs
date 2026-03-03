use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use blake3::Hasher;

pub struct Committer<'a> {
	db_conn: &'a Connection,
}

impl<'a> Committer<'a> {
	pub fn new(conn: &'a Connection) -> Self {
		Self { db_conn: conn }
	}

	pub fn create_commit(&self, message: &str) -> Result<String> {
		// 1. Fetch currently staged files (files added via 'mox add')
		// For this MVP, we consider all files currently in 'mod_files' as 'staged'
		let mut stmt = self.db_conn.prepare(
			"SELECT relative_path, blob_hash FROM mod_files"
		)?;

		let file_entries: Vec<(String, String)> = stmt.query_map([], |row| {
			Ok((row.get(0)?, row.get(1)?))
		})?.collect::<Result<Vec<_>, _>>()?;

		if file_entries.is_empty() {
			anyhow::bail!("Nothing to commit (use 'mox add' first)");
		}

		// 2. Generate unique Commit Hash based on contents + message
		let mut hasher = Hasher::new();
		hasher.update(message.as_bytes());
		for (path, hash) in &file_entries {
			hasher.update(path.as_bytes());
			hasher.update(hash.as_bytes());
		}
		let commit_hash = hasher.finalize().to_string();

		// 3. Insert Commit Record (Transaction)
		let tx = self.db_conn.unchecked_transaction()?;

		tx.execute(
			"INSERT INTO commits (message, hash) VALUES (?, ?)",
			params![message, commit_hash],
		)?;
		let commit_id = tx.last_insert_rowid();

		// 4. Link all files to this specific commit
		for (path, blob_hash) in file_entries {
			tx.execute(
				"INSERT INTO commit_contents (commit_id, blob_hash, relative_path) VALUES (?, ?, ?)",
				params![commit_id, blob_hash, path],
			)?;
		}

		tx.commit().context("Failed to finalize commit transaction")?;
		// Clear the staging area so the next commit doesn't duplicate these entries
		self.db_conn.execute("DELETE FROM mod_files", [])?;

		Ok(commit_hash)
	}
}