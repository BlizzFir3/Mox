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
		// 1. Fetch currently staged files (The Proposed State)
		let mut stmt = self.db_conn.prepare("SELECT relative_path, blob_hash FROM mod_files")?;
		let mut staged_files: Vec<(String, String)> = stmt.query_map([], |row| {
			Ok((row.get(0)?, row.get(1)?))
		})?.collect::<Result<Vec<_>, _>>()?;

		// Sort to ensure deterministic hashing and comparison
		staged_files.sort_by(|a, b| a.0.cmp(&b.0));

		// 2. Fetch the state of the last commit (The Old State)
		let current_branch = crate::core::branch::get_current_branch()?;
		let mut stmt = self.db_conn.prepare("SELECT last_commit_hash FROM branches WHERE name = ?")?;
		let last_hash: Option<String> = stmt.query_row([&current_branch], |row| row.get(0)).unwrap_or(None);

		let mut old_files: Vec<(String, String)> = Vec::new();
		if let Some(hash) = &last_hash {
			let mut stmt = self.db_conn.prepare(
				"SELECT relative_path, blob_hash FROM commit_contents
                 INNER JOIN commits ON commits.id = commit_contents.commit_id
                 WHERE commits.hash = ?"
			)?;
			old_files = stmt.query_map([hash], |row| Ok((row.get(0)?, row.get(1)?)))?
				.collect::<Result<Vec<_>, _>>()?;
			old_files.sort_by(|a, b| a.0.cmp(&b.0));
		}

		// 3. The "Vanilla" Check: If Staged == Old, nothing actually changed!
		if staged_files == old_files {
			anyhow::bail!("No changes detected. The staging area is identical to the last commit.");
		}

		// 4. Generate Deterministic Commit Hash
		let mut hasher = Hasher::new();
		hasher.update(message.as_bytes());
		for (path, hash) in &staged_files {
			hasher.update(path.as_bytes());
			hasher.update(hash.as_bytes());
		}
		let commit_hash = hasher.finalize().to_string();

		// 5. Database Transaction (Atomic Save)
		let tx = self.db_conn.unchecked_transaction()?;

		tx.execute(
			"INSERT INTO commits (message, hash) VALUES (?, ?)",
			params![message, commit_hash],
		)?;
		let commit_id = tx.last_insert_rowid();

		for (path, blob_hash) in staged_files {
			tx.execute(
				"INSERT INTO commit_contents (commit_id, blob_hash, relative_path) VALUES (?, ?, ?)",
				params![commit_id, blob_hash, path],
			)?;
		}

		// Update the active branch pointer to this new commit
		tx.execute(
			"UPDATE branches SET last_commit_hash = ? WHERE name = ?",
			params![commit_hash, current_branch],
		)?;

		tx.commit().context("Failed to finalize commit transaction")?;

		// SENIOR NOTE: Notice we NO LONGER clear `mod_files`.
		// Like Git, the staging area (index) should now exactly match HEAD.

		Ok(commit_hash)
	}
}