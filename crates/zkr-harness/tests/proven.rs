#![cfg(all(feature = "evm", feature = "svm"))]

//! The catalog may only claim a parity it can actually run. Every vector named
//! by a `proven_by` edge must load and drive both audited adapters to one shared
//! verdict, so a catalogued equivalence is never asserted beyond what the
//! committed vectors demonstrate.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use zkr_catalog::load_dir as load_catalog;
use zkr_harness::{Expected, evm, load_file, svm};

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
fn every_catalog_referenced_vector_drives_both_adapters_to_one_verdict() {
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
        let evm = evm::verify(&vector)
            .unwrap_or_else(|err| panic!("evm adapter should run `{name}`: {err:?}"));
        let svm = svm::verify(&vector)
            .unwrap_or_else(|err| panic!("svm adapter should run `{name}`: {err:?}"));
        assert_eq!(
            evm, svm,
            "EVM and SVM disagreed on catalog-referenced vector `{name}`"
        );
        assert_eq!(
            evm,
            vector.expected == Expected::Accept,
            "the adapters reached the wrong verdict on `{name}`"
        );
    }
}
