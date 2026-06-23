//! Loading test vectors from disk.

use std::fs;
use std::path::{Path, PathBuf};

use crate::model::Vector;

/// A vector together with the path it was loaded from.
#[derive(Debug, Clone)]
pub struct LoadedVector {
    /// Path of the source file.
    pub path: PathBuf,
    /// The parsed vector.
    pub vector: Vector,
}

/// An error encountered while loading a vector from disk.
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
    /// A file was not valid vector TOML.
    #[error("failed to parse {path}: {source}")]
    Parse {
        /// Path that failed to parse.
        path: PathBuf,
        /// Underlying deserialization error.
        source: toml::de::Error,
    },
}

/// Parses a single vector from its TOML representation.
pub fn parse_vector(toml_str: &str) -> Result<Vector, toml::de::Error> {
    toml::from_str(toml_str)
}

/// Loads a single `vector.toml`.
pub fn load_file(path: &Path) -> Result<LoadedVector, LoadError> {
    let text = fs::read_to_string(path).map_err(|source| LoadError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let vector = parse_vector(&text).map_err(|source| LoadError::Parse {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(LoadedVector {
        path: path.to_path_buf(),
        vector,
    })
}

/// Loads every `<root>/<name>/vector.toml`.
///
/// Subdirectories without a `vector.toml` are skipped, so documentation and other
/// files may sit alongside the vectors. Entries are returned in a stable,
/// path-sorted order.
pub fn load_dir(root: &Path) -> Result<Vec<LoadedVector>, LoadError> {
    read_sorted(root)?
        .into_iter()
        .map(|entry| entry.join("vector.toml"))
        .filter(|candidate| candidate.is_file())
        .map(|candidate| load_file(&candidate))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SAMPLE_VECTOR;
    use crate::model::{Expected, Primitive, ProofSystem};

    #[test]
    fn parses_a_valid_vector() {
        let vector = parse_vector(SAMPLE_VECTOR).expect("sample vector should parse");
        assert_eq!(vector.proof_system, ProofSystem::Groth16);
        assert_eq!(vector.primitive, Primitive::Bn254);
        assert_eq!(vector.expected, Expected::Accept);
        assert_eq!(vector.vk.ic.len(), 2);
    }

    #[test]
    fn rejects_unknown_field() {
        let toml = format!("extra = true\n{SAMPLE_VECTOR}");
        assert!(parse_vector(&toml).is_err());
    }

    #[test]
    fn rejects_non_hex_element() {
        let toml = SAMPLE_VECTOR.replace(
            "0x0000000000000000000000000000000000000000000000000000000000000001",
            "0xZZ",
        );
        assert!(parse_vector(&toml).is_err());
    }

    #[test]
    fn rejects_element_without_prefix() {
        let toml = SAMPLE_VECTOR.replace(
            "\"0x0000000000000000000000000000000000000000000000000000000000000001\"",
            "\"0000000000000000000000000000000000000000000000000000000000000001\"",
        );
        assert!(parse_vector(&toml).is_err());
    }

    #[test]
    fn rejects_unknown_proof_system() {
        let toml = SAMPLE_VECTOR.replace("groth16", "plonk");
        assert!(parse_vector(&toml).is_err());
    }
}
