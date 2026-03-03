use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use rusqlite::Connection;
use std::path::{Path, PathBuf};

mod core;
mod database;

#[derive(Parser)]
#[command(name = "mox")]
#[command(about = "A high-performance, Git-like version control for game mods", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new mox repository in the current directory
    Init,
    /// Add files to the staging area (index)
    Add {
        /// The path to the file or directory to add (defaults to current directory)
        #[arg(default_value = ".")]
        path: String
    },
    /// Record changes to the repository as a permanent snapshot
    Commit {
        /// Commit message describing the changes
        #[arg(short, long)]
        message: String
    },
    /// Show the commit history for the current profile
    Log,
    /// Show the working tree status (modified, untracked files)
    Status,
    /// Restore the working directory to a specific commit or switch profiles
    Checkout {
        /// The branch name or the commit hash prefix to restore
        name_or_hash: String
    },
    /// List all profiles, or create a new one if a name is provided
    Branch {
        /// Name of the new profile (branch) to create
        name: Option<String>,
    },
    /// Clean untracked files from the working directory
    Clean {
        /// Force deletion without dry-run
        #[arg(long)]
        force: bool,
    }
}

fn main() -> Result<()> {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Internal repository paths
    let mox_db_path = ".mox/mox.db";

    // CONFIGURATION: Global storage path for deduplicated blobs
    // TODO: For a production-grade setup, this should be extracted to a .env file or global config
    let store_path = PathBuf::from("F:/Mox_Global_Storage/blobs");

    // Command Routing
    match &cli.command {
        Commands::Init => {
            // Setup the directory structure and database
            core::repo::init_repository()?;
            // Ensure the HEAD pointer is initialized to the default branch
            core::branch::set_current_branch("main")?;
        }
        _ => {
            // Guard: Ensure we are executing inside a valid Mox repository
            if !Path::new(mox_db_path).exists() {
                anyhow::bail!("Fatal: Not a mox repository. Run 'mox init' first.");
            }

            // Establish database connection
            let conn = Connection::open(mox_db_path)
                .context("Failed to open local mox database")?;

            // SENIOR FIX: Auto-Migration
            // Ensure the database schema is always up-to-date before running any command.
            // This safely adds new tables (like 'branches') if they are missing.
            database::schema::init_db(&conn)
                .context("Failed to verify/update database schema")?;

            // Execute the requested command
            match &cli.command {
                Commands::Add { path } => {
                    let importer = core::mod_importer::ModImporter::new(&conn, store_path);
                    importer.import_all(Path::new(path))?;
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
                    core::log::show_log(&conn)?;
                }
                Commands::Status => {
                    core::status::show_status(&conn)?;
                }
                Commands::Checkout { name_or_hash } => {
                    core::checkout::restore_commit(&conn, name_or_hash, &store_path)?;
                }
                Commands::Branch { name } => {
                    if let Some(branch_name) = name {
                        core::branch::create_branch(&conn, branch_name)?;
                    } else {
                        core::branch::list_branches(&conn)?;
                    }
                }
                Commands::Clean { force } => {
                    core::clean::clean_untracked(&conn, !force)?;
                }
                _ => unreachable!(),
            }
        }
    }

    Ok(())
}