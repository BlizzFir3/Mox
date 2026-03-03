use std::fs;
use std::path::Path;
use anyhow::{Context, Result};
use crate::database::schema;
use rusqlite::Connection;

pub fn init_repository() -> Result<()> {
	let mox_path = Path::new(".mox");

	// Check if repository already exists to prevent accidental override
	if mox_path.exists() {
		anyhow::bail!("A mox repository already exists in this directory.");
	}

	// Create internal structure
	fs::create_dir_all(mox_path.join("blobs"))
		.context("Failed to create blobs directory")?;

	// Initialize the internal database
	let db_path = mox_path.join("mox.db");
	let conn = Connection::open(db_path)?;
	schema::init_db(&conn).context("Failed to initialize database schema")?;

	println!("Initialized empty Mox repository in {:?}", fs::canonicalize(mox_path)?);
	Ok(())
}