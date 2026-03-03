use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::fs;
use std::path::Path;

pub fn restore_commit(conn: &Connection, target: &str, store_path: &Path) -> Result<()> {
	// --- 1. CLEANUP OF CURRENT STATE ---
	// Before switching branches, we must identify currently active files and physically
	// remove them from the disk to prevent the game engine from loading inactive mods.
	if let Ok(current_branch) = crate::core::branch::get_current_branch() {
		if let Ok(Some(current_hash)) = conn.query_row(
			"SELECT last_commit_hash FROM branches WHERE name = ?",
			[&current_branch],
			|row| row.get::<_, Option<String>>(0)
		) {
			// Retrieve the list of files tracked by the current commit
			let mut stmt = conn.prepare(
				"SELECT relative_path FROM commit_contents
                 INNER JOIN commits ON commits.id = commit_contents.commit_id
                 WHERE commits.hash = ?"
			)?;

			let old_files = stmt.query_map([current_hash], |row| row.get::<_, String>(0))?;

			// Rigorously remove them from the working directory (Mods folder)
			for old_file in old_files.flatten() {
				let path_to_remove = Path::new(&old_file);
				if path_to_remove.exists() {
					// We ignore the error if the user already manually deleted the file
					let _ = fs::remove_file(path_to_remove);
				}
			}
		}
	}

	// --- 2. TARGET RESOLUTION ---
	// Determine if the target is a branch name or a raw commit hash
	let mut stmt_branch = conn.prepare("SELECT last_commit_hash FROM branches WHERE name = ?")?;
	let resolved_hash = match stmt_branch.query_row([target], |row| row.get::<_, Option<String>>(0)) {
		Ok(Some(hash)) => {
			crate::core::branch::set_current_branch(target)?;
			println!("Switched to profile '{}'", target);
			hash
		}
		Ok(None) => anyhow::bail!("Profile '{}' exists but has no commits yet.", target),
		Err(_) => target.to_string() // Fallback: Treat as a raw commit hash
	};

	let mut stmt = conn.prepare("SELECT id, hash FROM commits WHERE hash LIKE ? LIMIT 1")?;
	let (commit_id, full_hash): (i64, String) = stmt.query_row(
		[format!("{}%", resolved_hash)],
		|row| Ok((row.get(0)?, row.get(1)?))
	).context("Commit not found")?;

	println!("Checking out {}...", &full_hash[..7]);

	// --- 3. TARGET STATE RESTORATION ---
	// Fetch the file manifest for the target commit
	let mut stmt = conn.prepare(
		"SELECT relative_path, blob_hash FROM commit_contents WHERE commit_id = ?"
	)?;

	let files = stmt.query_map(params![commit_id], |row| {
		Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
	})?;

	let mut count = 0;
	for file_result in files {
		let (rel_path, blob_hash) = file_result?;
		let target_path = Path::new(&rel_path);
		let blob_source = store_path.join(&blob_hash);

		// Ensure parent directories exist (e.g., Mods/Clothes/...)
		if let Some(parent) = target_path.parent() {
			fs::create_dir_all(parent)?;
		}

		// PHYSICAL COPY: The game engine requires real files to function correctly.
		// This avoids symlink permission issues and ensures 100% compatibility.
		fs::copy(&blob_source, target_path)
			.with_context(|| format!("Failed to copy real file for {}", rel_path))?;

		count += 1;
	}

	// --- 4. DEFRAGMENTATION & CLEANUP ---
	// Prevent directory fragmentation by removing empty folders left behind
	// by the previous branch's file structure.
	crate::core::clean::clean_empty_directories(Path::new("."))?;

	// --- 5. SYNC STAGING AREA (INDEX) ---
	// Ensure the staging area perfectly mirrors the newly checked-out state
	conn.execute("DELETE FROM mod_files", [])?;
	conn.execute(
		"INSERT INTO mod_files (blob_hash, relative_path) 
         SELECT blob_hash, relative_path FROM commit_contents WHERE commit_id = ?",
		params![commit_id]
	)?;

	// --- 6. DEFRAGMENTATION & CLEANUP ---
	// Prevent directory fragmentation by removing empty folders...
	crate::core::clean::clean_empty_directories(Path::new("."))?;

	println!("Successfully deployed {} real files to your Mods folder.", count);
	Ok(())
}