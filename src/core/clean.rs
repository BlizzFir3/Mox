use anyhow::Result;
use rusqlite::Connection;
use std::fs;
use std::collections::HashSet;
use walkdir::WalkDir;

pub fn clean_untracked(conn: &Connection, dry_run: bool) -> Result<()> {
	// 1. Get all files belonging to the latest commit
	let mut stmt = conn.prepare(
		"SELECT relative_path FROM commit_contents 
         WHERE commit_id = (SELECT id FROM commits ORDER BY created_at DESC LIMIT 1)"
	)?;

	let tracked_files: HashSet<String> = stmt.query_map([], |row| {
		Ok(row.get::<_, String>(0)?)
	})?.collect::<Result<HashSet<_>, _>>()?;

	println!("Cleaning untracked files...");

	// 2. Scan and identify files to delete
	for entry in WalkDir::new(".").into_iter().filter_map(|e| e.ok()) {
		let path = entry.path();
		if path.is_dir() || path.starts_with("./.mox") { continue; }

		let rel_path = path.strip_prefix("./").unwrap_or(path).to_str().unwrap().to_string();

		if !tracked_files.contains(&rel_path) {
			if dry_run {
				println!("Would remove: {}", rel_path);
			} else {
				println!("Removing: {}", rel_path);
				fs::remove_file(path)?;
			}
		}
	}

	if dry_run {
		println!("\nThis was a dry run. Use --force to actually delete files.");
	}

	Ok(())
}