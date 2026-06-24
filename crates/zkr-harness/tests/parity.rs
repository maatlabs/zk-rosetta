#![cfg(all(feature = "evm", feature = "svm"))]

//! The Rosetta demonstration: one BN254 Groth16 statement, driven through the
//! audited verifiers of two ecosystems and shown to reach the *same* verdict on
//! the *same* bytes---accepting the real proof identically and rejecting a
//! tampered one identically. This cross-ecosystem agreement, not either verdict
//! on its own, is the property the harness exists to assert.

use std::path::Path;

use zkr_harness::{Vector, evm, load_file, parse_vector, svm};

const VECTOR_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../vectors/bn254-groth16-multiplier/vector.toml"
);

/// Drives `vector` through both audited verifiers and returns their shared
/// verdict, failing if the ecosystems disagree on the same statement.
fn parity_verdict(vector: &Vector) -> bool {
    let evm = evm::verify(vector).expect("evm adapter should run");
    let svm = svm::verify(vector).expect("svm adapter should run");
    assert_eq!(
        evm, svm,
        "EVM and SVM verifiers disagreed on the same statement"
    );
    evm
}

#[test]
fn evm_and_svm_agree_to_accept_the_committed_proof() {
    let loaded = load_file(Path::new(VECTOR_PATH)).expect("committed vector should load");
    assert!(
        parity_verdict(&loaded.value),
        "both ecosystems must accept the committed proof"
    );
}

#[test]
fn evm_and_svm_agree_to_reject_a_tampered_proof() {
    let text = std::fs::read_to_string(VECTOR_PATH).expect("vector file should read");
    // The committed statement is 3 * 11 = 33 (0x21); re-check the same proof
    // against 34 (0x22), which both verifiers must reject.
    let tampered = text.replace(
        "0x0000000000000000000000000000000000000000000000000000000000000021",
        "0x0000000000000000000000000000000000000000000000000000000000000022",
    );
    assert_ne!(text, tampered, "the public input must actually change");
    let vector = parse_vector(&tampered).expect("tampered vector should parse");
    assert!(
        !parity_verdict(&vector),
        "both ecosystems must reject the tampered proof"
    );
}
