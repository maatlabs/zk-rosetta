//! Loading proposals from disk.

use std::fs;
use std::path::{Path, PathBuf};

use crate::model::Proposal;

/// A proposal together with the path it was loaded from.
#[derive(Debug, Clone)]
pub struct LoadedProposal {
    /// Path of the source file.
    pub path: PathBuf,
    /// The parsed proposal.
    pub proposal: Proposal,
}

/// An error encountered while loading the catalog from disk.
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    /// A directory or file could not be read.
    #[error("failed to read {path}: {source}")]
    Io {
        /// Path that could not be read.
        path: PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },
    /// A file was not valid proposal TOML.
    #[error("failed to parse {path}: {source}")]
    Parse {
        /// Path that failed to parse.
        path: PathBuf,
        /// Underlying deserialization error.
        source: toml::de::Error,
    },
}

/// Parses a single proposal from its TOML representation.
pub fn parse_proposal(toml_str: &str) -> Result<Proposal, toml::de::Error> {
    toml::from_str(toml_str)
}

/// Loads every `<root>/<ecosystem>/<id>.toml` proposal file under `root`.
///
/// Entries are returned in a stable, path-sorted order.
pub fn load_dir(root: &Path) -> Result<Vec<LoadedProposal>, LoadError> {
    read_sorted(root)?
        .into_iter()
        .filter(|p| p.is_dir())
        .flat_map(|dir| match read_sorted(&dir) {
            Ok(files) => files
                .into_iter()
                .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("toml"))
                .map(load_file)
                .collect(),
            Err(err) => vec![Err(err)],
        })
        .collect()
}

fn read_sorted(dir: &Path) -> Result<Vec<PathBuf>, LoadError> {
    let mut entries = fs::read_dir(dir)
        .map_err(|source| LoadError::Io {
            path: dir.to_path_buf(),
            source,
        })?
        .map(|entry| {
            entry
                .map(|entry| entry.path())
                .map_err(|source| LoadError::Io {
                    path: dir.to_path_buf(),
                    source,
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort();
    Ok(entries)
}

fn load_file(path: PathBuf) -> Result<LoadedProposal, LoadError> {
    let text = fs::read_to_string(&path).map_err(|source| LoadError::Io {
        path: path.clone(),
        source,
    })?;
    let proposal = parse_proposal(&text).map_err(|source| LoadError::Parse {
        path: path.clone(),
        source,
    })?;
    Ok(LoadedProposal { path, proposal })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Primitive;

    const VALID: &str = r#"
id = "EIP-197"
title = "Pairing"
ecosystem = "ethereum"
layer = "L1"
category = "primitive"
status = "final"
native_status = "Final"
primitive = "BN254"
enables = "Groth16 full verification"
spec = "https://eips.ethereum.org/EIPS/eip-197"
notes = "Pairing check."
"#;

    #[test]
    fn parses_a_valid_proposal() {
        let proposal = parse_proposal(VALID).expect("valid proposal should parse");
        assert_eq!(proposal.id, "EIP-197");
        assert_eq!(proposal.primitive, Some(Primitive::Bn254));
    }

    #[test]
    fn rejects_unknown_fields() {
        let toml = format!("{VALID}extra_field = true\n");
        assert!(parse_proposal(&toml).is_err());
    }

    #[test]
    fn rejects_missing_required_field() {
        let toml = VALID.replace("title = \"Pairing\"\n", "");
        assert!(parse_proposal(&toml).is_err());
    }

    #[test]
    fn rejects_unknown_enum_value() {
        let toml = VALID.replace("ecosystem = \"ethereum\"", "ecosystem = \"dogecoin\"");
        assert!(parse_proposal(&toml).is_err());
    }
}
