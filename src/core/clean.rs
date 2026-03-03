use anyhow::Result;
use rusqlite::Connection;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

/// Removes files from the working directory that are not tracked by the current commit.
pub fn clean_untracked(conn: &Connection, dry_run: bool) -> Result<()> {
	// 1. Fetch the manifest of the active commit
	let mut stmt = conn.prepare(
		"SELECT relative_path FROM commit_contents
         WHERE commit_id = (SELECT id FROM commits ORDER BY created_at DESC LIMIT 1)"
	)?;

	let tracked_files: HashSet<String> = stmt.query_map([], |row| {
		Ok(row.get::<_, String>(0)?)
	})?.collect::<Result<HashSet<_>, _>>()?;

	println!("Cleaning untracked files...");

	// 2. Scan the working directory for rogue files
	for entry in WalkDir::new(".").into_iter().filter_map(|e| e.ok()) {
		let path = entry.path();

		// Skip directories and the internal .mox state folder
		if path.is_dir() || path.components().any(|c| c.as_os_str() == ".mox") {
			continue;
		}

		let rel_path = path.strip_prefix("./").unwrap_or(path).to_str().unwrap().to_string();

		// If the file isn't in our current commit manifest, it's untracked
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

/// Recursively removes empty directories to prevent structural fragmentation.
/// Performs a bottom-up (post-order) traversal to ensure nested empty folders are cleared.
pub fn clean_empty_directories(root: &Path) -> Result<()> {
	// SENIOR DEV NOTE: contents_first(true) guarantees we visit children before parents.
	// This is a strict requirement for safely deleting nested folder structures.
	for entry in WalkDir::new(root).contents_first(true).into_iter().filter_map(|e| e.ok()) {
		let path = entry.path();

		// Safety Guard: NEVER touch the .mox infrastructure
		if path.components().any(|c| c.as_os_str() == ".mox") {
			continue;
		}

		if path.is_dir() {
			// fs::remove_dir is naturally safe: it will throw an OS error and fail
			// if the directory contains any files. We deliberately ignore the Result
			// because a failure just means the directory is legitimately in use.
			let _ = fs::remove_dir(path);
		}
	}

	Ok(())
}