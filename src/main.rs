use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use rusqlite::Connection;
use std::path::{Path, PathBuf};

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
    /// Add files to the staging area
    Add {
        #[arg(default_value = ".")]
        path: String
    },
    /// Record changes to the repository
    Commit {
        #[arg(short, long)]
        message: String
    },
    /// Show commit history
    Log,
    /// Show the working tree status
    Status,
    /// Restore a specific commit or switch to a profile
    Checkout {
        name_or_hash: String
    },
    /// List profiles, or create a new one if a name is provided
    Branch {
        /// Name of the new profile to create
        name: Option<String>,
    },
    /// Clean untracked files
    Clean {
        #[arg(long)]
        force: bool,
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mox_db_path = ".mox/mox.db";

    // CONFIGURATION: Set your global storage path here
    let store_path = PathBuf::from("F:/Mox_Global_Storage/blobs");

    match &cli.command {
        Commands::Init => {
            core::repo::init_repository()?;
            // Ensure HEAD is created on init
            core::branch::set_current_branch("main")?;
        }
        _ => {
            if !Path::new(mox_db_path).exists() {
                anyhow::bail!("Fatal: Not a mox repository. Run 'mox init' first.");
            }

            let conn = Connection::open(mox_db_path)?;

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
                _ => {}
            }
        }
    }

    Ok(())
}