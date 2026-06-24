//! Command-line tools for the zk-rosetta catalog.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use zkr_catalog::LoadedProposal;

#[derive(Parser)]
#[command(name = "zkr", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate the catalog dataset.
    Validate {
        /// Path to the catalog data directory.
        #[arg(long, default_value = "data")]
        data: PathBuf,
        /// Also check that every spec and implementation URL resolves.
        #[arg(long)]
        online: bool,
    },
    /// Print the JSON Schema for a catalog proposal.
    Schema,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> anyhow::Result<ExitCode> {
    match Cli::parse().command {
        Command::Validate { data, online } => validate(&data, online),
        Command::Schema => {
            println!("{}", zkr_catalog::schema_json()?);
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn validate(data: &Path, online: bool) -> anyhow::Result<ExitCode> {
    let loaded = zkr_catalog::load_dir(data)?;

    let mut problems = zkr_catalog::validate(&loaded)
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<String>>();
    if online {
        problems.extend(unreachable_links(&loaded));
    }

    if problems.is_empty() {
        println!("validated {} proposals: ok", loaded.len());
        Ok(ExitCode::SUCCESS)
    } else {
        problems
            .iter()
            .for_each(|problem| eprintln!("invalid: {problem}"));
        eprintln!("{} problem(s) found", problems.len());
        Ok(ExitCode::FAILURE)
    }
}

fn unreachable_links(loaded: &[LoadedProposal]) -> Vec<String> {
    loaded
        .iter()
        .flat_map(|entry| {
            let proposal = &entry.value;
            std::iter::once(proposal.spec.clone())
                .chain(proposal.sources.iter().cloned())
                .chain(
                    proposal
                        .implementations
                        .iter()
                        .flat_map(|i| std::iter::once(i.url.clone()).chain(i.audit_ref.clone())),
                )
                .filter_map(|url| match reachable(&url) {
                    Ok(()) => None,
                    Err(err) => Some(format!("`{}`: unreachable {url}: {err}", proposal.id)),
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Resolves a URL, retrying a few times so a single transient network failure
/// does not flag an otherwise-reachable link.
fn reachable(url: &str) -> Result<(), ureq::Error> {
    const ATTEMPTS: usize = 3;
    std::iter::repeat_with(|| ureq::get(url).call())
        .take(ATTEMPTS)
        .enumerate()
        .find_map(|(attempt, result)| match result {
            Ok(_) => Some(Ok(())),
            Err(_) if attempt + 1 < ATTEMPTS => {
                std::thread::sleep(std::time::Duration::from_millis(500));
                None
            }
            Err(err) => Some(Err(err)),
        })
        .unwrap_or(Ok(()))
}
