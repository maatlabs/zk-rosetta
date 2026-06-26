#![cfg(all(feature = "evm", feature = "filecoin"))]

//! Exercises the BLS12-381 signature parity end-to-end against the committed
//! vector: the EIP-2537 pairing precompile and Filecoin's audited
//! `bls-signatures` verifier must reach the same verdict on the same bytes, both
//! accepting the real signature and both rejecting it against the wrong message.

use std::path::Path;

use zkr_harness::{Statement, evm, filecoin, load_file};

const VECTOR_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../vectors/bls12-381-bls-signature/vector.toml"
);

#[test]
fn evm_and_filecoin_accept_the_committed_signature() {
    let vector = load_file(Path::new(VECTOR_PATH))
        .expect("committed vector should load")
        .value;
    let evm = evm::verify(&vector).expect("evm adapter should run");
    let filecoin = filecoin::verify(&vector).expect("filecoin adapter should run");
    assert!(evm, "EIP-2537 must accept the committed signature");
    assert!(
        filecoin,
        "bls-signatures must accept the committed signature"
    );
}

#[test]
fn evm_and_filecoin_reject_a_signature_against_the_wrong_message() {
    let mut vector = load_file(Path::new(VECTOR_PATH))
        .expect("committed vector should load")
        .value;
    // Re-check the signature against a different valid G2 point (the signature
    // itself); both audited verifiers must reject it, and must agree.
    let Statement::BlsSignature(bls) = &mut vector.statement else {
        panic!("the committed vector should be a BLS signature statement");
    };
    bls.message_hash = bls.signature.clone();

    let evm = evm::verify(&vector).expect("evm adapter should run");
    let filecoin = filecoin::verify(&vector).expect("filecoin adapter should run");
    assert!(!evm, "EIP-2537 must reject the wrong-message signature");
    assert!(
        !filecoin,
        "bls-signatures must reject the wrong-message signature"
    );
    assert_eq!(evm, filecoin, "the adapters must agree on rejection");
}
