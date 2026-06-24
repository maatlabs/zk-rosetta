//! Loading proposals from disk.

use std::path::Path;

pub use zkr_core::LoadError;
use zkr_core::{Loaded, load_file, read_sorted};

use crate::model::Proposal;

/// A proposal together with the path it was loaded from.
pub type LoadedProposal = Loaded<Proposal>;

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
                .map(|p| load_file::<Proposal>(&p))
                .collect(),
            Err(err) => vec![Err(err)],
        })
        .collect()
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
