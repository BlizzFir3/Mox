use std::fs;
use std::path::Path;
use anyhow::{Context, Result};
use crate::database::schema;
use rusqlite::Connection;

pub fn init_repository() -> Result<()> {
	let mox_path = Path::new(".mox");

	if mox_path.exists() {
		anyhow::bail!("A mox repository already exists in this directory.");
	}

	// 1. Create the directory
	fs::create_dir_all(mox_path.join("blobs"))
		.context("Failed to create blobs directory")?;

	// 2. Hide the directory (Windows specific)
	hide_directory(mox_path)?;

	// 3. Initialize the internal database
	let db_path = mox_path.join("mox.db");
	let conn = Connection::open(db_path)?;
	schema::init_db(&conn).context("Failed to initialize database schema")?;

	println!("Initialized empty Mox repository (hidden) in {:?}", fs::canonicalize(mox_path)?);
	Ok(())
}

/// Sets the hidden attribute on Windows systems
fn hide_directory(path: &std::path::Path) -> anyhow::Result<()> {
	#[cfg(windows)]
	{
		use std::process::Command;
		// +H sets Hidden, +I sets Not Content Indexed (saves CPU/IO for mods)
		Command::new("attrib")
			.arg("+h")
			.arg("+i")
			.arg(path)
			.status()?;
	}
	Ok(())
}