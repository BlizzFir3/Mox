use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::fs;
use std::path::Path;

pub fn restore_commit(conn: &Connection, target: &str, store_path: &Path) -> Result<()> {
	// 1. Resolve target: Is it a branch name or a raw commit hash?
	let mut stmt_branch = conn.prepare("SELECT last_commit_hash FROM branches WHERE name = ?")?;

	let resolved_hash = match stmt_branch.query_row([target], |row| row.get::<_, Option<String>>(0)) {
		Ok(Some(hash)) => {
			// It is a valid branch with a commit
			crate::core::branch::set_current_branch(target)?;
			println!("Switched to profile '{}'", target);
			hash
		}
		Ok(None) => {
			anyhow::bail!("Profile '{}' exists but has no commits yet.", target);
		}
		Err(_) => {
			// It's not a branch in the DB, assume the user provided a raw commit hash
			target.to_string()
		}
	};

	// 2. Find the internal commit ID
	let mut stmt = conn.prepare("SELECT id, hash FROM commits WHERE hash LIKE ? LIMIT 1")?;
	let (commit_id, full_hash): (i64, String) = stmt.query_row(
		[format!("{}%", resolved_hash)],
		|row| Ok((row.get(0)?, row.get(1)?))
	).context("Commit not found")?;

	println!("Checking out {}...", &full_hash[..7]);

	// 3. Fetch all files associated with this commit
	let mut stmt = conn.prepare(
		"SELECT relative_path, blob_hash FROM commit_contents WHERE commit_id = ?"
	)?;

	let files = stmt.query_map(params![commit_id], |row| {
		Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
	})?;

	// 4. Restore files using Symbolic Links (supports multi-drive)
	for file_result in files {
		let (rel_path, blob_hash) = file_result?;
		let target_path = Path::new(&rel_path);
		let blob_source = store_path.join(&blob_hash);

		if let Some(parent) = target_path.parent() {
			fs::create_dir_all(parent)?;
		}

		if target_path.exists() {
			fs::remove_file(target_path)?;
		}

		#[cfg(windows)]
		std::os::windows::fs::symlink_file(&blob_source, target_path)
			.with_context(|| format!("Failed to create symlink for {}. Run as Admin.", rel_path))?;

		#[cfg(unix)]
		std::os::unix::fs::symlink(&blob_source, target_path)?;
	}

	println!("Successfully restored state.");
	Ok(())
}