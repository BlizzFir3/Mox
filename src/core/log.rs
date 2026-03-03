use anyhow::Result;
use rusqlite::Connection;

pub fn show_log(conn: &Connection) -> Result<()> {
	let mut stmt = conn.prepare(
		"SELECT hash, message, created_at FROM commits ORDER BY created_at DESC"
	)?;

	let commit_iter = stmt.query_map([], |row| {
		Ok((
			row.get::<_, String>(0)?,
			row.get::<_, String>(1)?,
			row.get::<_, String>(2)?,
		))
	})?;

	println!("\x1b[1m--- Mox Commit History ---\x1b[0m");

	for commit in commit_iter {
		let (hash, message, date) = commit?;
		// Professional CLI formatting (Yellow for hash, Cyan for date)
		println!("\x1b[33mcommit {}\x1b[0m", hash);
		println!("\x1b[36mDate:\x1b[0m    {}", date);
		println!("    {}\n", message);
	}

	Ok(())
}