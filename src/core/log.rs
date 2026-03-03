use anyhow::Result;
use rusqlite::Connection;
use chrono::{DateTime, Utc};

pub struct CommitRecord {
	pub hash: String,
	pub message: String,
	pub created_at: String,
}

pub fn show_log(conn: &Connection) -> Result<()> {
	let mut stmt = conn.prepare(
		"SELECT hash, message, created_at FROM commits ORDER BY created_at DESC"
	)?;

	let commit_iter = stmt.query_map([], |row| {
		Ok(CommitRecord {
			hash: row.get(0)?,
			message: row.get(1)?,
			created_at: row.get(2)?,
		})
	})?;

	println!("--- Commit History ---");

	let mut count = 0;
	for commit in commit_iter {
		let c = commit?;
		// Parse the SQLite datetime string
		let date = DateTime::parse_from_rfc3339(&c.created_at)
			.map(|dt| dt.with_timezone(&Utc).format("%Y-%m-%d %H:%M:%S").to_string())
			.unwrap_or(c.created_at);

		println!("\x1b[33mcommit {}\x1b[0m", c.hash); // Yellow hash
		println!("Date:    {}", date);
		println!("\n    {}\n", c.message);
		count += 1;
	}

	if count == 0 {
		println!("No commits found. Use 'mox commit' to create one.");
	}

	Ok(())
}