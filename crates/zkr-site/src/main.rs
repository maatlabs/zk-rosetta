//! Static-site generator for the zk-rosetta catalog.

mod render;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "zkr-site", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Render the catalog dataset into a static site.
    Build {
        /// Path to the catalog data directory.
        #[arg(long, default_value = "data")]
        data: PathBuf,
        /// Directory the generated site is written to.
        #[arg(long, default_value = "dist")]
        out: PathBuf,
    },
}

fn main() -> ExitCode {
    let Cli { command } = Cli::parse();
    let Command::Build { data, out } = command;
    match render::build(&data, &out) {
        Ok(count) => {
            println!("rendered {count} proposals to {}", out.display());
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}
