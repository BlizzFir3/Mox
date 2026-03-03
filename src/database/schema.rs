use rusqlite::{Connection, Result};

pub fn init_db(conn: &Connection) -> Result<()> {
	// Local-first optimisation
	conn.execute_batch("
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA foreign_keys = ON;
    ")?;

	conn.execute(
		"CREATE TABLE IF NOT EXISTS blobs (
            hash TEXT PRIMARY KEY,
            size_bytes INTEGER NOT NULL,
            added_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
		[],
	)?;

	conn.execute(
		"CREATE TABLE IF NOT EXISTS mods (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            version TEXT,
            source_path TEXT
        )",
		[],
	)?;

	conn.execute(
		"CREATE TABLE IF NOT EXISTS mod_files (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            mod_id INTEGER NOT NULL,
            blob_hash TEXT NOT NULL,
            relative_path TEXT NOT NULL,
            FOREIGN KEY(mod_id) REFERENCES mods(id),
            FOREIGN KEY(blob_hash) REFERENCES blobs(hash)
        )",
		[],
	)?;

	conn.execute_batch("
    CREATE TABLE IF NOT EXISTS commits (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        parent_id INTEGER,
        message TEXT NOT NULL,
        hash TEXT UNIQUE NOT NULL, -- The hash of the commit itself
        created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
        FOREIGN KEY(parent_id) REFERENCES commits(id)
    );

    CREATE TABLE IF NOT EXISTS commit_contents (
        commit_id INTEGER NOT NULL,
        blob_hash TEXT NOT NULL,
        relative_path TEXT NOT NULL,
        PRIMARY KEY (commit_id, relative_path),
        FOREIGN KEY(commit_id) REFERENCES commits(id),
        FOREIGN KEY(blob_hash) REFERENCES blobs(hash)
    );
")?;

	Ok(())
}