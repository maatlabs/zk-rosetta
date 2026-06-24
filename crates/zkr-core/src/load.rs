//! Loading typed records from TOML files on disk.

use std::fs;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;

/// A deserialized value together with the path it was loaded from.
#[derive(Debug, Clone)]
pub struct Loaded<T> {
    /// Path of the source file.
    pub path: PathBuf,
    /// The parsed value.
    pub value: T,
}

/// An error encountered while loading a record from disk.
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
    /// A file was not valid TOML for the target type.
    #[error("failed to parse {path}: {source}")]
    Parse {
        /// Path that failed to parse.
        path: PathBuf,
        /// Underlying deserialization error.
        source: toml::de::Error,
    },
}

/// The entries of `dir`, returned in a stable, path-sorted order.
pub fn read_sorted(dir: &Path) -> Result<Vec<PathBuf>, LoadError> {
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

/// Reads `path` and deserializes its TOML contents into `T`.
pub fn load_file<T: DeserializeOwned>(path: &Path) -> Result<Loaded<T>, LoadError> {
    let text = fs::read_to_string(path).map_err(|source| LoadError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let value = toml::from_str(&text).map_err(|source| LoadError::Parse {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(Loaded {
        path: path.to_path_buf(),
        value,
    })
}
