mod store;

use std::io::{self, Write};
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use store::Store;

/// pmux — associate filesystem paths with tags, then list them back.
#[derive(Parser)]
#[command(
    name = "pmux",
    version,
    about = "Path multiplexer: tag paths and list them back"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Associate a path with a tag
    Add {
        /// Tag to file the path under
        tag: String,
        /// Path to associate (relative paths are resolved against the current directory)
        path: String,
    },
    /// List all tags, or the paths of one tag
    List {
        /// Tag whose paths to list; omit to list every tag
        tag: Option<String>,
    },
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("pmux: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> io::Result<()> {
    let cli = Cli::parse();
    let store_path = Store::default_path()?;
    let mut store = Store::load(&store_path)?;

    match cli.command {
        Command::Add { tag, path } => {
            if tag.trim().is_empty() {
                return Err(io::Error::other("tag must not be empty"));
            }
            let normalized = store::normalize(&path)?;
            if store.add(&tag, normalized.clone()) {
                store.save(&store_path)?;
                println!("added {normalized} to {tag}");
            } else {
                println!("{normalized} is already in {tag}");
            }
        }
        Command::List { tag: None } => {
            let stdout = io::stdout();
            let mut out = stdout.lock();
            for tag in store.tags() {
                writeln!(out, "{tag}")?;
            }
        }
        Command::List { tag: Some(tag) } => {
            let paths = store
                .paths(&tag)
                .ok_or_else(|| io::Error::other(format!("unknown tag: {tag}")))?;
            let stdout = io::stdout();
            let mut out = stdout.lock();
            for path in paths {
                writeln!(out, "{path}")?;
            }
        }
    }
    Ok(())
}
