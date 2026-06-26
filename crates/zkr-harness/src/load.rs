//! Loading test vectors from disk.

use std::path::Path;

pub use zkr_core::LoadError;
use zkr_core::{Loaded, read_sorted};

use crate::model::Vector;

/// A vector together with the path it was loaded from.
pub type LoadedVector = Loaded<Vector>;

/// Parses a single vector from its TOML representation.
pub fn parse_vector(toml_str: &str) -> Result<Vector, toml::de::Error> {
    toml::from_str(toml_str)
}

/// Loads a single `vector.toml`.
pub fn load_file(path: &Path) -> Result<LoadedVector, LoadError> {
    zkr_core::load_file(path)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Expected, Primitive, Statement};
    use crate::{SAMPLE_BLS_VECTOR, SAMPLE_VECTOR};

    #[test]
    fn parses_a_valid_vector() {
        let vector = parse_vector(SAMPLE_VECTOR).expect("sample vector should parse");
        assert_eq!(vector.primitive, Primitive::Bn254);
        assert_eq!(vector.expected, Expected::Accept);
        let Statement::Groth16(groth16) = vector.statement else {
            panic!("sample vector should be a Groth16 statement");
        };
        assert_eq!(groth16.vk.ic.len(), 2);
    }

    #[test]
    fn parses_a_bls_signature_statement() {
        let vector = parse_vector(SAMPLE_BLS_VECTOR).expect("bls sample should parse");
        assert_eq!(vector.primitive, Primitive::Bls12381);
        assert!(
            matches!(vector.statement, Statement::BlsSignature(_)),
            "the bls sample should deserialize into a BLS signature statement"
        );
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
    fn rejects_unknown_statement_kind() {
        let toml = SAMPLE_VECTOR.replace("groth16", "plonk");
        assert!(parse_vector(&toml).is_err());
    }
}
