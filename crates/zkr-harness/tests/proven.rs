#![cfg(all(feature = "evm", feature = "svm", feature = "filecoin"))]

//! The catalog may only claim a parity it can actually run. Every vector named
//! by a `proven_by` edge must load and drive the audited adapters its statement
//! targets to one shared verdict, so a catalogued equivalence is never asserted
//! beyond what the committed vectors demonstrate.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use zkr_catalog::load_dir as load_catalog;
use zkr_harness::{Expected, Statement, evm, filecoin, load_file, svm};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every distinct vector name referenced by a catalog `proven_by` edge.
fn referenced_vectors() -> BTreeSet<String> {
    load_catalog(&workspace_root().join("data"))
        .expect("the catalog should load")
        .into_iter()
        .flat_map(|loaded| loaded.value.proven_by)
        .collect()
}

#[test]
fn every_catalog_referenced_vector_drives_its_adapters_to_one_verdict() {
    let referenced = referenced_vectors();
    assert!(
        !referenced.is_empty(),
        "expected the catalog to reference at least one proving vector"
    );
    for name in referenced {
        let path = workspace_root()
            .join("vectors")
            .join(&name)
            .join("vector.toml");
        let vector = load_file(&path)
            .unwrap_or_else(|err| panic!("referenced vector `{name}` should load: {err:?}"))
            .value;

        // Each statement names the ecosystem pair its primitive can be driven on:
        // Groth16 BN254 across the EVM and SVM, a BLS signature across the EVM and
        // Filecoin (the Solana BLS syscalls are not yet activated upstream).
        let (left, right) = match &vector.statement {
            Statement::Groth16(_) => (
                evm::verify(&vector)
                    .unwrap_or_else(|err| panic!("evm adapter should run `{name}`: {err:?}")),
                svm::verify(&vector)
                    .unwrap_or_else(|err| panic!("svm adapter should run `{name}`: {err:?}")),
            ),
            Statement::BlsSignature(_) => (
                evm::verify(&vector)
                    .unwrap_or_else(|err| panic!("evm adapter should run `{name}`: {err:?}")),
                filecoin::verify(&vector)
                    .unwrap_or_else(|err| panic!("filecoin adapter should run `{name}`: {err:?}")),
            ),
        };

        assert_eq!(
            left, right,
            "the adapters disagreed on catalog-referenced vector `{name}`"
        );
        assert_eq!(
            left,
            vector.expected == Expected::Accept,
            "the adapters reached the wrong verdict on `{name}`"
        );
    }
}
