#![cfg(feature = "evm")]

//! Exercises the EVM adapter end-to-end against the committed BN254 Groth16
//! vector: the audited verifier must accept the real proof and reject the same
//! proof checked against the wrong public input.

use std::path::Path;

use zkr_harness::{evm, load_file, parse_vector};

const VECTOR_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../vectors/bn254-groth16-multiplier/vector.toml"
);

#[test]
fn evm_accepts_the_committed_multiplier_proof() {
    let loaded = load_file(Path::new(VECTOR_PATH)).expect("committed vector should load");
    assert!(
        evm::verify(&loaded.value).expect("evm adapter should run"),
        "audited Groth16 verifier must accept the committed proof"
    );
}

#[test]
fn evm_rejects_a_proof_against_the_wrong_public_input() {
    let text = std::fs::read_to_string(VECTOR_PATH).expect("vector file should read");
    // The committed statement is 3 * 11 = 33 (0x21); re-check the same proof
    // against 34 (0x22), which a correct verifier must reject.
    let tampered = text.replace(
        "0x0000000000000000000000000000000000000000000000000000000000000021",
        "0x0000000000000000000000000000000000000000000000000000000000000022",
    );
    assert_ne!(text, tampered, "the public input must actually change");
    let vector = parse_vector(&tampered).expect("tampered vector should parse");
    assert!(
        !evm::verify(&vector).expect("evm adapter should run"),
        "verifier must reject the proof against the wrong public input"
    );
}
