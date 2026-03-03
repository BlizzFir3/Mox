# Mox 📦

**A high-performance, Git-like version control system for game mods.**

Mox is a CLI tool designed to manage massive local modding environments (like *The Sims 4*, *Skyrim*, or *Fallout*). It uses a Content-Addressable Storage (CAS) engine to deduplicate files, allowing you to create infinite mod profiles (branches) without duplicating data on your disk.

By leveraging symbolic links, Mox allows you to store hundreds of gigabytes of mods on a cheap, high-capacity HDD, while seamlessly linking them to your game's fast SSD.

## ✨ Key Features

* **Zero-Duplication (CAS):** Files are hashed using BLAKE3. If 10 different mod profiles use the same 2GB texture pack, it is only stored on your disk once.
* **Multi-Drive Architecture:** Store your blobs globally (e.g., `F:\MoxStorage`) and deploy them instantly to your game directory (e.g., `C:\Games\Sims 4\Mods`) via OS-level Symbolic Links.
* **Instant Profile Switching:** Create branches for different playthroughs (e.g., `Vanilla+`, `Chaos`, `Realistic`) and switch between them in seconds.
* **Atomic Commits:** Powered by a bundled SQLite database with Write-Ahead Logging (WAL) for safe, transactional state snapshots.
* **Smart Filtering:** Ignore dynamic game caches and temporary files using a `.moxignore` file.

## 🚀 Installation & Setup

### Prerequisites

* [Rust & Cargo](https://rustup.rs/) installed.
* **Windows Users:** Developer Mode must be enabled, OR the terminal must be run as **Administrator** to allow Mox to create Symbolic Links across drives.

### Build

```bash
git clone https://github.com/yourusername/mox.git
cd mox
cargo build --release

```

Move the compiled binary (`target/release/mox.exe`) to a directory in your system's `PATH`, or place it directly in your game's `Mods` folder.

### Global Store Configuration

By default, the global blob storage is configured in `src/main.rs`. Ensure this path points to your high-capacity drive before compiling:

```rust
let store_path = PathBuf::from("F:/Mox_Global_Storage/blobs");

```

## 📖 Core Workflow

Navigate to your game's mod directory (e.g., `C:\Users\Name\Documents\Electronic Arts\The Sims 4\Mods`) and start tracking your mods:

### 1. Initialize the Repository

Creates a hidden `.mox` directory and sets up the local SQLite index.

```bash
mox init

```

### 2. Ignore Cache Files (Optional but Recommended)

Create a `.moxignore` file in the root of your Mods folder:

```text
localthumbcache.package
lastException.txt
notify.glob

```

### 3. Stage and Commit Mods

Track your current mod folder state and save it as a snapshot.

```bash
mox add .
mox commit -m "Stable base setup with core scripts"

```

### 4. Create and Switch Profiles (Branches)

Want to try a risky modpack without ruining your stable setup? Create a new profile.

```bash
# Create a new profile based on the current state
mox branch TestingPack

# Switch to the new profile
mox checkout TestingPack

```

### 5. Check Status & Clean

Check what files have changed, been added, or removed since your last commit.

```bash
mox status

```

If you manually added some trash files or broke a mod, you can instantly revert your folder to the exact state of your last commit:

```bash
mox clean --force

```

## 🛠️ Command Reference

| Command | Description |
| --- | --- |
| `mox init` | Initializes a new local Mox repository. |
| `mox status` | Shows modified and untracked files in the working directory. |
| `mox add <path>` | Hashes and stages files for the next commit. Moves data to global store. |
| `mox commit -m "<msg>"` | Creates a permanent snapshot of the staged files. |
| `mox log` | Displays the commit history for the current profile. |
| `mox branch` | Lists all profiles. The active profile is highlighted. |
| `mox branch <name>` | Creates a new profile pointing to the current commit. |
| `mox checkout <name>` | Clears the directory and restores symlinks for the target profile or hash. |
| `mox clean --force` | Removes any untracked files from the working directory. |

## 🏗️ Technical Architecture

* **Language:** Rust (2021 Edition)
* **Database:** SQLite (`rusqlite`) for fast, local indexing of manifests and file paths.
* **Hashing:** `blake3` for multi-threaded, cryptographic-grade file fingerprinting.
* **CLI Engine:** `clap` for robust argument parsing.