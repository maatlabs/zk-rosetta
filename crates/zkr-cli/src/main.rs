//! Command-line tools for the zk-rosetta catalog.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use zkr_catalog::{Finding, LoadedProposal, Proposal, Source};

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
    /// Check catalog entries against their upstream proposal repositories.
    Drift {
        /// Path to the catalog data directory.
        #[arg(long, default_value = "data")]
        data: PathBuf,
        /// Output format for the drift report.
        #[arg(long, value_enum, default_value_t = Format::Human)]
        format: Format,
    },
}

/// The output format for the drift report.
#[derive(Clone, Copy, ValueEnum)]
enum Format {
    /// One line per finding, with a summary, for interactive use.
    Human,
    /// A JSON array of findings, for the scheduled freshness action.
    Json,
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
        Command::Drift { data, format } => drift(&data, format),
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

fn drift(data: &Path, format: Format) -> anyhow::Result<ExitCode> {
    let loaded = zkr_catalog::load_dir(data)?;
    let sources = zkr_catalog::sources();
    let findings = loaded
        .iter()
        .flat_map(|entry| check(&entry.value, &sources))
        .collect::<Vec<Finding>>();

    let drifted = findings.iter().filter(|finding| finding.is_drift()).count();
    match format {
        Format::Json => println!("{}", serde_json::to_string_pretty(&findings)?),
        Format::Human => {
            findings.iter().for_each(|finding| eprintln!("{finding}"));
            eprintln!(
                "checked {} proposals: {drifted} drifted, {} note(s)",
                loaded.len(),
                findings.len() - drifted
            );
        }
    }

    Ok(if drifted == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// Checks a single proposal against its ecosystem's upstream source.
fn check(proposal: &Proposal, sources: &[Box<dyn Source>]) -> Vec<Finding> {
    let Some(source) = zkr_catalog::source_for(sources, proposal.ecosystem) else {
        return vec![Finding::NoSource {
            id: proposal.id.clone(),
            ecosystem: proposal.ecosystem,
        }];
    };
    let Some(url) = source.document_url(proposal) else {
        return vec![Finding::NoLocator {
            id: proposal.id.clone(),
        }];
    };
    match fetch(&url) {
        Ok(body) => zkr_catalog::compare(source, proposal, &url, &body),
        Err(err) => vec![Finding::Unreachable {
            id: proposal.id.clone(),
            url,
            error: err.to_string(),
        }],
    }
}

/// Fetches a URL body, retrying a few times so a single transient network
/// failure does not look like upstream drift.
fn fetch(url: &str) -> Result<String, ureq::Error> {
    const ATTEMPTS: usize = 3;
    std::iter::repeat_with(|| {
        ureq::get(url)
            .call()
            .and_then(|mut response| response.body_mut().read_to_string())
    })
    .take(ATTEMPTS)
    .enumerate()
    .find_map(|(attempt, result)| match result {
        Ok(body) => Some(Ok(body)),
        Err(_) if attempt + 1 < ATTEMPTS => {
            std::thread::sleep(std::time::Duration::from_millis(500));
            None
        }
        Err(err) => Some(Err(err)),
    })
    .expect("the final attempt always yields a result")
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
