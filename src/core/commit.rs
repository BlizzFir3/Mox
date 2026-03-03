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
		let mut stmt = self.db_conn.prepare(
			"SELECT relative_path, blob_hash FROM mod_files"
		)?;

		let file_entries: Vec<(String, String)> = stmt.query_map([], |row| {
			Ok((row.get(0)?, row.get(1)?))
		})?.collect::<Result<Vec<_>, _>>()?;

		if file_entries.is_empty() {
			anyhow::bail!("Nothing to commit (use 'mox add' first)");
		}

		let mut hasher = Hasher::new();
		hasher.update(message.as_bytes());
		for (path, hash) in &file_entries {
			hasher.update(path.as_bytes());
			hasher.update(hash.as_bytes());
		}
		let commit_hash = hasher.finalize().to_string();

		let tx = self.db_conn.unchecked_transaction()?;

		tx.execute(
			"INSERT INTO commits (message, hash) VALUES (?, ?)",
			params![message, commit_hash],
		)?;
		let commit_id = tx.last_insert_rowid();

		for (path, blob_hash) in file_entries {
			tx.execute(
				"INSERT INTO commit_contents (commit_id, blob_hash, relative_path) VALUES (?, ?, ?)",
				params![commit_id, blob_hash, path],
			)?;
		}

		tx.execute("DELETE FROM mod_files", [])?;

		// SENIOR MOVE: Update the active branch pointer to this new commit
		let current_branch = crate::core::branch::get_current_branch()?;
		tx.execute(
			"UPDATE branches SET last_commit_hash = ? WHERE name = ?",
			params![commit_hash, current_branch],
		)?;

		tx.commit().context("Failed to finalize commit transaction")?;

		Ok(commit_hash)
	}
}