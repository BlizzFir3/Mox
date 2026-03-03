// src/main.rs
use anyhow::{Result};
use clap::{Parser, Subcommand};
use rusqlite::Connection;
use std::path::Path;

mod core;
mod database;

#[derive(Parser)]
#[command(name = "mox")]
#[command(about = "A Git-like version control for game mods", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new mox repository in the current folder
    Init,
    /// Add current folder files to staging
    Add {
        /// Path to add (use "." for current folder)
        #[arg(default_value = ".")]
        path: String,
    },
    /// Create a new snapshot of staged files
    Commit {
        /// Commit message
        #[arg(short, long)]
        message: String,
    },
    /// List changes between disk and last commit
    Status,
    /// Restore the folder to a specific commit state
    Checkout {
        /// Hash prefix of the commit
        hash: String,
    },
    /// Show commit history
    Log,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mox_db_path = ".mox/mox.db";

    match &cli.command {
        Commands::Init => {
            core::repo::init_repository()?;
        }
        _ => {
            // Ensure .mox exists in the current working directory
            if !Path::new(mox_db_path).exists() {
                anyhow::bail!("Fatal: Not a mox repository. Run 'mox init' first.");
            }

            let conn = Connection::open(mox_db_path)?;

            match &cli.command {
                Commands::Add { path } => {
                    let source_path = Path::new(path);
                    let storage_path = Path::new(".mox/blobs");

                    let importer = core::mod_importer::ModImporter::new(&conn, storage_path.to_path_buf());
                    importer.import_mod("StagedFiles", source_path)?;
                    println!("Successfully indexed files from '{}'", path);
                }
                Commands::Commit { message } => {
                    let committer = core::commit::Committer::new(&conn);
                    match committer.create_commit(message) {
                        Ok(hash) => println!("[main {}] {}", &hash[..7], message),
                        Err(e) => eprintln!("Commit failed: {}", e),
                    }
                }
                Commands::Log => {
                    core::log::show_log(&conn)?; // Changed from core::status to core::log
                }
                Commands::Status => {
                    core::status::show_status(&conn)?;
                }
                Commands::Checkout { hash } => {
                    core::checkout::restore_commit(&conn, hash)?;
                }
                _ => unreachable!(),
            }
        }
    }

    Ok(())
}