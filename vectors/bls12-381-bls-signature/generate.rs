//! Regenerates `vector.toml` for the `bls12-381-bls-signature` vector.
//!
//! Signs a fixed message under a deterministically derived BLS12-381 key using
//! Filecoin's audited `bls-signatures` (its `blst`/`blstrs` backend), self-checks
//! the signature, and prints the public key, signature, and hashed message as
//! uncompressed affine coordinates in `[c0, c1]` (real-part-first) order. It also
//! prints the negated G1 generator the EVM adapter pairs the signature against.
//!
//! Run via `generate.sh`, which supplies the pinned dependencies.

use bls_signatures::{PrivateKey, hash, verify};
use blstrs::{G1Affine, G2Affine, G2Projective};
use group::prime::PrimeCurveAffine;

const IKM: &[u8] = b"zk-rosetta::bls12-381-bls-signature::v1 :: deterministic IKM";
const MESSAGE: &[u8] =
    b"zk-rosetta proves BLS12-381 signature parity: EIP-2537 == Filecoin bls-signatures";

fn fp(bytes: [u8; 48]) -> String {
    format!("0x{}", hex::encode(bytes))
}

fn g1_inline(name: &str, point: &G1Affine) {
    println!(
        "{name} = {{ x = \"{}\", y = \"{}\" }}",
        fp(point.x().to_bytes_be()),
        fp(point.y().to_bytes_be())
    );
}

fn g2_table(header: &str, point: &G2Affine) {
    println!("[{header}]");
    println!(
        "x = [\"{}\", \"{}\"]",
        fp(point.x().c0().to_bytes_be()),
        fp(point.x().c1().to_bytes_be())
    );
    println!(
        "y = [\"{}\", \"{}\"]",
        fp(point.y().c0().to_bytes_be()),
        fp(point.y().c1().to_bytes_be())
    );
}

fn main() {
    let secret_key = PrivateKey::new(IKM);
    let public_key = secret_key.public_key();
    let signature = secret_key.sign(MESSAGE);
    let message_hash: G2Projective = hash(MESSAGE);

    assert!(
        verify(&signature, &[message_hash], &[public_key]),
        "the generated signature must verify under bls-signatures"
    );

    println!("primitive = \"BLS12-381\"");
    println!("expected = \"accept\"");
    println!();
    println!("[statement.bls-signature]");
    g1_inline("public_key", &public_key.as_affine());
    println!();
    g2_table(
        "statement.bls-signature.signature",
        &G2Affine::from(signature),
    );
    println!();
    g2_table("statement.bls-signature.message_hash", &G2Affine::from(message_hash));

    let neg_g1 = -G1Affine::generator();
    eprintln!("# EVM adapter constant -- negated G1 generator:");
    eprintln!("#   x = {}", fp(neg_g1.x().to_bytes_be()));
    eprintln!("#   y = {}", fp(neg_g1.y().to_bytes_be()));
}
