// src/main.rs
use anyhow::{Context, Result};
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
    /// Initialize a new mox repository
    Init,
    /// Add files to the staging area (index)
    Add {
        /// The path to the file or directory to add
        path: String,
    },
    /// Record changes to the repository
    Commit {
        /// Commit message
        #[arg(short, long)]
        message: String,
    },
    /// Show the working tree status
    Status,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mox_db_path = ".mox/mox.db";

    match &cli.command {
        Commands::Init => {
            core::repo::init_repository()?;
        }
        _ => {
            // Guard: Ensure we are in a mox repo for all other commands
            if !Path::new(mox_db_path).exists() {
                anyhow::bail!("Fatal: Not a mox repository (or any of the parent directories): .mox");
            }

            let conn = Connection::open(mox_db_path)?;

            match &cli.command {
                Commands::Add { path } => {
                    let source_path = Path::new(path);
                    let storage_path = Path::new(".mox/blobs");

                    // Logic: Walk directory, hash files, and record in DB
                    let importer = core::mod_importer::ModImporter::new(&conn, storage_path.to_path_buf());
                    importer.import_mod("StagedFiles", source_path)?;
                    println!("Successfully added '{}' to staging.", path);
                }
                Commands::Commit { message } => {
                    let committer = core::commit::Committer::new(&conn);
                    match committer.create_commit(message) {
                        Ok(hash) => println!("[main {}] {}", &hash[..7], message),
                        Err(e) => eprintln!("Commit failed: {}", e),
                    }
                }
                Commands::Status => {
                    // Logic: Compare current files on disk with the last commit
                    core::status::show_status(&conn)?;
                }
                _ => unreachable!(),
            }
        }
    }

    Ok(())
}