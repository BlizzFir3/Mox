use anyhow::Result; // Cleaned unused Context
use rusqlite::Connection;
use std::collections::HashMap;
use walkdir::WalkDir;
use crate::core::hasher;

pub fn show_status(conn: &Connection) -> Result<()> {
	// 1. Get HEAD files
	let mut stmt = conn.prepare(
		"SELECT relative_path, blob_hash FROM commit_contents
         WHERE commit_id = (SELECT id FROM commits ORDER BY created_at DESC LIMIT 1)"
	)?;

	let head_files: HashMap<String, String> = stmt.query_map([], |row| {
		Ok((row.get(0)?, row.get(1)?))
	})?.collect::<Result<HashMap<_, _>, _>>()?;

	println!("On branch 'main'");

	let mut modified = Vec::new();
	let mut untracked = Vec::new();

	// 2. Scan Working Directory
	for entry in WalkDir::new(".").into_iter().filter_map(|e| e.ok()) {
		let path = entry.path();
		if path.is_dir() || path.starts_with("./.mox") { continue; }

		let rel_path = path.strip_prefix("./").unwrap_or(path).to_str().unwrap().to_string();

		match head_files.get(&rel_path) {
			Some(old_hash) => {
				let new_hash = hasher::hash_file(path)?;
				if &new_hash != old_hash {
					modified.push(rel_path);
				}
			}
			None => {
				untracked.push(rel_path);
			}
		}
	}

	// 3. Display (Fixed Borrows)
	if !modified.is_empty() {
		println!("\nChanges not staged for commit:");
		for path in &modified { // Added & to borrow
			println!("\tmodified:   {}", path);
		}
	}

	if !untracked.is_empty() {
		println!("\nUntracked files:");
		for path in &untracked { // Added & to borrow
			println!("\t{}", path);
		}
	}

	if modified.is_empty() && untracked.is_empty() {
		println!("\nnothing to commit, working tree clean");
	}

	Ok(())
}