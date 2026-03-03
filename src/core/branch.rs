use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::fs;
use std::path::Path;

/// Reads the currently active branch (profile) from .mox/HEAD
pub fn get_current_branch() -> Result<String> {
	let head_path = Path::new(".mox/HEAD");
	if head_path.exists() {
		let content = fs::read_to_string(head_path)?;
		Ok(content.trim().to_string())
	} else {
		// Fallback to default branch if HEAD file is missing
		Ok("main".to_string())
	}
}

/// Updates the HEAD pointer to a new branch
pub fn set_current_branch(name: &str) -> Result<()> {
	fs::write(".mox/HEAD", name).context("Failed to write HEAD file")?;
	Ok(())
}

/// Creates a new branch pointing to the active branch's current commit
pub fn create_branch(conn: &Connection, name: &str) -> Result<()> {
	let current_branch = get_current_branch()?;

	// 1. Get the commit hash of the current branch
	let mut stmt = conn.prepare("SELECT last_commit_hash FROM branches WHERE name = ?")?;
	let current_hash: Option<String> = stmt.query_row([&current_branch], |row| row.get(0))
		.unwrap_or(None);

	// 2. Insert the new branch into the database
	conn.execute(
		"INSERT INTO branches (name, last_commit_hash) VALUES (?, ?)",
		params![name, current_hash],
	).context("Branch already exists or database error")?;

	println!("Created new profile: {}", name);
	Ok(())
}

/// Lists all branches and highlights the currently active one
pub fn list_branches(conn: &Connection) -> Result<()> {
	let current_branch = get_current_branch()?;

	let mut stmt = conn.prepare("SELECT name, last_commit_hash FROM branches ORDER BY name")?;
	let branches = stmt.query_map([], |row| {
		Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
	})?;

	println!("\x1b[1m--- Profiles ---\x1b[0m");
	for b in branches {
		let (name, hash) = b?;
		let hash_display = hash.unwrap_or_else(|| "no commits".to_string());
		let short_hash = if hash_display.len() > 7 { &hash_display[..7] } else { &hash_display };

		// Print active branch in Green with an asterisk
		if name == current_branch {
			println!("* \x1b[32m{} ({})\x1b[0m", name, short_hash);
		} else {
			println!("  {} ({})", name, short_hash);
		}
	}

	Ok(())
}