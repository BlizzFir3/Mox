use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::fs;
use std::path::Path;

pub fn restore_commit(conn: &Connection, target: &str, store_path: &Path) -> Result<()> {
	// 1. Resolve branch name or hash
	let mut stmt_branch = conn.prepare("SELECT last_commit_hash FROM branches WHERE name = ?")?;

	let resolved_hash = match stmt_branch.query_row([target], |row| row.get::<_, Option<String>>(0)) {
		Ok(Some(hash)) => {
			crate::core::branch::set_current_branch(target)?;
			println!("Switched to profile '{}'", target);
			hash
		}
		Ok(None) => {
			anyhow::bail!("Profile '{}' exists but has no commits yet.", target);
		}
		Err(_) => target.to_string()
	};

	// 2. Retrieve commit ID
	let mut stmt = conn.prepare("SELECT id, hash FROM commits WHERE hash LIKE ? LIMIT 1")?;
	let (commit_id, full_hash): (i64, String) = stmt.query_row(
		[format!("{}%", resolved_hash)],
		|row| Ok((row.get(0)?, row.get(1)?))
	).context("Commit not found")?;

	println!("Checking out {}...", &full_hash[..7]);

	// 3. Retrieve file list for this commit
	let mut stmt = conn.prepare(
		"SELECT relative_path, blob_hash FROM commit_contents WHERE commit_id = ?"
	)?;

	let files = stmt.query_map(params![commit_id], |row| {
		Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
	})?;

	// 4. Restoration (Real physical copy to ensure game compatibility)
	let mut count = 0;
	for file_result in files {
		let (rel_path, blob_hash) = file_result?;
		let target_path = Path::new(&rel_path);
		let blob_source = store_path.join(&blob_hash);

		// Create subdirectories if necessary (e.g., Mods/Clothes/...)
		if let Some(parent) = target_path.parent() {
			fs::create_dir_all(parent)?;
		}

		// If a file already exists at this location, overwrite it cleanly
		if target_path.exists() {
			fs::remove_file(target_path)?;
		}

		// PHYSICAL COPY: The game will see a real normal file.
		// This is the safest operation, requiring no Administrator rights.
		fs::copy(&blob_source, target_path)
			.with_context(|| format!("Failed to copy real file for {}", rel_path))?;

		count += 1;
	}

	println!("Successfully deployed {} real files to your Mods folder.", count);
	Ok(())
}