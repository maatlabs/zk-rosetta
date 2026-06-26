//! Command-line tools for the zk-rosetta catalog.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use rayon::prelude::*;
use zkr_catalog::{Fetched, Finding, LoadedProposal, Primitive, Proposal, Source};
use zkr_harness::LoadedVector;

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
        /// Path to the committed test-vector directory.
        #[arg(long, default_value = "vectors")]
        vectors: PathBuf,
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
    /// Inspect the committed parity test vectors.
    Vectors {
        #[command(subcommand)]
        command: VectorCommand,
    },
}

/// Operations over the committed parity test vectors.
#[derive(Subcommand)]
enum VectorCommand {
    /// Check that every committed vector is structurally well-formed.
    Validate {
        /// Path to the committed test-vector directory.
        #[arg(long, default_value = "vectors")]
        vectors: PathBuf,
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
        Command::Validate {
            data,
            vectors,
            online,
        } => validate(&data, &vectors, online),
        Command::Schema => {
            println!("{}", zkr_catalog::schema_json()?);
            Ok(ExitCode::SUCCESS)
        }
        Command::Drift { data, format } => drift(&data, format),
        Command::Vectors { command } => match command {
            VectorCommand::Validate { vectors } => validate_vectors(&vectors),
        },
    }
}

/// Validates the committed test vectors, failing if any is structurally malformed.
fn validate_vectors(root: &Path) -> anyhow::Result<ExitCode> {
    let loaded = zkr_harness::load_dir(root)?;
    let problems = vector_problems(&loaded);

    if problems.is_empty() {
        println!("validated {} vectors: ok", loaded.len());
        Ok(ExitCode::SUCCESS)
    } else {
        problems
            .iter()
            .for_each(|problem| eprintln!("invalid: {problem}"));
        eprintln!("{} problem(s) found", problems.len());
        Ok(ExitCode::FAILURE)
    }
}

/// Returns one message per structural problem in the loaded vectors, in the
/// loader's stable path-sorted order.
fn vector_problems(loaded: &[LoadedVector]) -> Vec<String> {
    loaded
        .iter()
        .flat_map(|entry| {
            zkr_harness::validate(&entry.value)
                .into_iter()
                .map(move |error| format!("{}: {error}", entry.path.display()))
        })
        .collect()
}

fn validate(data: &Path, vectors: &Path, online: bool) -> anyhow::Result<ExitCode> {
    let loaded = zkr_catalog::load_dir(data)?;
    let vector_primitives = vector_primitives(vectors)?;

    let mut problems = zkr_catalog::validate(&loaded, &vector_primitives)
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<String>>();
    if online {
        problems.extend(unreachable_links(&loaded, |url| {
            reachable(url).map_err(|err| err.to_string())
        }));
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

/// Maps each committed vector's directory name to the primitive it exercises.
///
/// This is the authority catalog validation checks every `proven_by` reference
/// against, so a proposal cannot claim a parity no committed vector backs.
fn vector_primitives(root: &Path) -> anyhow::Result<HashMap<String, Primitive>> {
    Ok(zkr_harness::load_dir(root)?
        .into_iter()
        .filter_map(|loaded| {
            loaded
                .path
                .parent()
                .and_then(Path::file_name)
                .and_then(|name| name.to_str())
                .map(|name| (name.to_string(), loaded.value.primitive))
        })
        .collect())
}

/// Returns one message per catalog URL that `check` rejects, in catalog order.
///
/// Each URL is an independent network round-trip, so the checks run in parallel,
/// but the output order matches a sequential pass over the dataset: the link set
/// is materialized in catalog order first, then resolved with an order-preserving
/// parallel collect.
fn unreachable_links(
    loaded: &[LoadedProposal],
    check: impl Fn(&str) -> Result<(), String> + Sync,
) -> Vec<String> {
    let links: Vec<(&str, &str)> =
        loaded
            .iter()
            .flat_map(|entry| {
                let proposal = &entry.value;
                std::iter::once(proposal.spec.as_str())
                    .chain(proposal.sources.iter().map(String::as_str))
                    .chain(proposal.implementations.iter().flat_map(|i| {
                        std::iter::once(i.url.as_str()).chain(i.audit_ref.as_deref())
                    }))
                    .map(move |url| (proposal.id.as_str(), url))
            })
            .collect();

    links
        .par_iter()
        .filter_map(|(id, url)| {
            check(url)
                .err()
                .map(|err| format!("`{id}`: unreachable {url}: {err}"))
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
    zkr_catalog::resolve(source, proposal, &url, fetch)
}

/// Fetches a URL body for drift resolution. A 404 is reported immediately (it
/// is deterministic and is the relocation signal `resolve` chases), while other
/// failures are retried a few times so a single transient blip does not look
/// like upstream drift.
fn fetch(url: &str) -> Fetched {
    const ATTEMPTS: usize = 3;
    std::iter::repeat_with(|| {
        ureq::get(url)
            .call()
            .and_then(|mut response| response.body_mut().read_to_string())
    })
    .take(ATTEMPTS)
    .enumerate()
    .find_map(|(attempt, result)| match result {
        Ok(body) => Some(Fetched::Body(body)),
        Err(ureq::Error::StatusCode(404)) => Some(Fetched::NotFound),
        Err(_) if attempt + 1 < ATTEMPTS => {
            std::thread::sleep(std::time::Duration::from_millis(500));
            None
        }
        Err(err) => Some(Fetched::Error(err.to_string())),
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

#[cfg(test)]
mod tests {
    use zkr_catalog::parse_proposal;

    use super::*;

    fn loaded(toml: &str) -> LoadedProposal {
        let value = parse_proposal(toml).expect("fixture parses");
        let path = format!("data/ethereum/{}.toml", value.id.to_ascii_lowercase()).into();
        LoadedProposal { path, value }
    }

    #[test]
    fn online_failures_are_reported_in_catalog_order_across_every_url_class() {
        // Two proposals: the first carries a spec, a source, and an
        // implementation with an audit_ref; the second only a spec. Failing one
        // url from different proposals and classes proves the check visits every
        // class (including the optional audit_ref) and that the parallel pass
        // returns failures in catalog order, not completion order.
        let first = loaded(
            "id = \"EIP-1\"\ntitle = \"t\"\necosystem = \"ethereum\"\nlayer = \"L1\"\n\
             category = \"primitive\"\nstatus = \"final\"\nnative_status = \"x\"\n\
             enables = \"e\"\nspec = \"https://a/spec\"\nsources = [\"https://a/src\"]\n\
             notes = \"n\"\n\n[[implementations]]\nname = \"i\"\nlanguage = \"rust\"\n\
             url = \"https://a/impl\"\naudited = true\naudit_ref = \"https://a/audit\"\n",
        );
        let second = loaded(
            "id = \"EIP-2\"\ntitle = \"t\"\necosystem = \"ethereum\"\nlayer = \"L1\"\n\
             category = \"primitive\"\nstatus = \"final\"\nnative_status = \"x\"\n\
             enables = \"e\"\nspec = \"https://b/spec\"\nnotes = \"n\"\n",
        );
        let failing = ["https://a/audit", "https://b/spec"];
        let problems = unreachable_links(&[first, second], |url| {
            if failing.contains(&url) {
                Err("boom".to_string())
            } else {
                Ok(())
            }
        });
        assert_eq!(
            problems,
            vec![
                "`EIP-1`: unreachable https://a/audit: boom".to_string(),
                "`EIP-2`: unreachable https://b/spec: boom".to_string(),
            ]
        );
    }

    #[test]
    fn vector_problems_pass_on_committed_vectors_and_flag_a_malformed_one() {
        let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../vectors"));
        let loaded = zkr_harness::load_dir(root).expect("committed vectors should load");
        assert!(!loaded.is_empty(), "expected at least one committed vector");
        assert!(
            vector_problems(&loaded).is_empty(),
            "committed vectors must validate clean"
        );

        let mut broken = loaded;
        broken[0].value.vk.ic.pop();
        let problems = vector_problems(&broken);
        let path = broken[0].path.display().to_string();
        assert!(
            problems.iter().any(|problem| problem.contains(&path)),
            "expected a problem naming `{path}`, got {problems:?}"
        );
    }
}
