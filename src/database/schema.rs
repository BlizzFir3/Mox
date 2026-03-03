use rusqlite::{Connection, Result};

pub fn init_db(conn: &Connection) -> Result<()> {
	conn.execute_batch("
        PRAGMA journal_mode = WAL;
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS blobs (
            hash TEXT PRIMARY KEY,
            size_bytes INTEGER NOT NULL,
            added_at TEXT DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'NOW'))
        );

        CREATE TABLE IF NOT EXISTS mod_files (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            blob_hash TEXT NOT NULL,
            relative_path TEXT NOT NULL,
            FOREIGN KEY(blob_hash) REFERENCES blobs(hash)
        );

        CREATE TABLE IF NOT EXISTS commits (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            message TEXT NOT NULL,
            hash TEXT UNIQUE NOT NULL,
            created_at TEXT DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'NOW'))
        );

        CREATE TABLE IF NOT EXISTS commit_contents (
            commit_id INTEGER NOT NULL,
            blob_hash TEXT NOT NULL,
            relative_path TEXT NOT NULL,
            PRIMARY KEY (commit_id, relative_path),
            FOREIGN KEY(commit_id) REFERENCES commits(id)
        );

        CREATE TABLE IF NOT EXISTS branches (
            name TEXT PRIMARY KEY,
            last_commit_hash TEXT,
            FOREIGN KEY(last_commit_hash) REFERENCES commits(hash)
        );

        INSERT OR IGNORE INTO branches (name) VALUES ('main');
    ")?;
	Ok(())
}