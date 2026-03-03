use anyhow::Result;
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};
use walkdir::WalkDir;
use crate::core::ignore::MoxIgnore;

pub fn show_status(conn: &Connection) -> Result<()> {
	let current_branch = crate::core::branch::get_current_branch().unwrap_or_else(|_| "main".to_string());
	println!("On profile '{}'\n", current_branch);

	// 1. Fetch the state of the last commit (Manifest)
	let mut stmt = conn.prepare("SELECT last_commit_hash FROM branches WHERE name = ?")?;
	let last_commit_hash: Option<String> = stmt.query_row([&current_branch], |row| row.get(0)).unwrap_or(None);

	let mut tracked_files = HashMap::new();
	if let Some(hash) = last_commit_hash {
		let mut stmt = conn.prepare(
			"SELECT relative_path, blob_hash FROM commit_contents 
             INNER JOIN commits ON commits.id = commit_contents.commit_id 
             WHERE commits.hash = ?"
		)?;
		let rows = stmt.query_map([hash], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?;
		for row in rows {
			let (path, blob_hash) = row?;
			tracked_files.insert(path, blob_hash);
		}
	}

	// 2. Scan the Working Directory (Disk)
	let ignore = MoxIgnore::load();
	let mut disk_files = HashSet::new();

	for entry in WalkDir::new(".").into_iter().filter_map(|e| e.ok()) {
		let path = entry.path();
		if path.is_dir() || ignore.is_ignored(path) || path.components().any(|c| c.as_os_str() == ".mox") {
			continue;
		}

		let rel_path = path.strip_prefix("./").unwrap_or(path).to_str().unwrap().to_string();
		disk_files.insert(rel_path);
	}

	// 3. Compare State
	let mut deleted = Vec::new();
	let mut untracked = Vec::new();

	// Check for Deleted files
	for tracked_path in tracked_files.keys() {
		if !disk_files.contains(tracked_path) {
			deleted.push(tracked_path.clone());
		}
	}

	// Check for Untracked (New) files
	for disk_path in &disk_files {
		if !tracked_files.contains_key(disk_path) {
			untracked.push(disk_path.clone());
		}
	}

	// 4. Display Output
	if deleted.is_empty() && untracked.is_empty() {
		println!("Nothing to commit, working directory is clean.");
		return Ok(());
	}

	if !deleted.is_empty() {
		println!("Deleted files:");
		for f in deleted {
			println!("  \x1b[31m{}\x1b[0m", f); // Red text
		}
		println!();
	}

	if !untracked.is_empty() {
		println!("Untracked files:");
		for f in untracked {
			println!("  \x1b[32m{}\x1b[0m", f); // Green text
		}
		println!();
	}

	Ok(())
}