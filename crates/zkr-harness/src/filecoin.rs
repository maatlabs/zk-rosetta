//! Filecoin parity adapter.
//!
//! Drives Filecoin's audited [`bls-signatures`](https://github.com/filecoin-project/bls-signatures)
//! BLS12-381 verifier---its `blst`/`blstrs` backend, the one Filecoin runs in
//! production---over a BLS signature vector. The adapter authors no
//! cryptography: it lays the ecosystem-neutral coordinates into `blstrs`'s
//! uncompressed point encoding, parses them with `blstrs`'s own deserializer
//! (which rejects points off the curve or outside the prime-order subgroup), and
//! hands them to `bls_signatures::verify`, which evaluates the pairing relation
//! `e(g1, signature) == e(public_key, message_hash)`.

use bls_signatures::{PublicKey, Signature, verify as bls_verify};
use blstrs::{G1Affine, G1Projective, G2Affine, G2Projective};

use crate::model::{BlsSignature, Element, G1, G2, Primitive, Statement, Vector};

/// A fault in running the Filecoin adapter, as opposed to a verdict on the proof.
///
/// A rejected signature is reported as `Ok(false)` by [`verify`]; these variants
/// signal that the harness itself could not produce a verdict.
#[derive(Debug, thiserror::Error)]
pub enum FilecoinError {
    /// The vector is not the BLS12-381 signature shape this adapter verifies.
    #[error("vector is not a BLS12-381 signature vector")]
    Unsupported,
    /// A coordinate was wider than the 48-byte BLS12-381 field encoding.
    #[error("{0} carries a field element wider than 48 bytes")]
    ElementTooWide(&'static str),
    /// A point was off the curve or outside the prime-order subgroup.
    #[error("{0} is not a BLS12-381 point in the prime-order subgroup")]
    InvalidPoint(&'static str),
}

/// Runs Filecoin's audited BLS verifier over `vector`, returning whether it accepted.
///
/// `Ok(true)` means `bls_signatures::verify` accepted the signature, `Ok(false)`
/// means it rejected it. An `Err` signals a harness or setup fault, not a verdict
/// on the signature itself.
pub fn verify(vector: &Vector) -> Result<bool, FilecoinError> {
    match &vector.statement {
        Statement::BlsSignature(bls) if vector.primitive == Primitive::Bls12381 => verify_bls(bls),
        _ => Err(FilecoinError::Unsupported),
    }
}

fn verify_bls(bls: &BlsSignature) -> Result<bool, FilecoinError> {
    let public_key = PublicKey::from(G1Projective::from(g1(&bls.public_key, "public_key")?));
    let signature = Signature::from(g2(&bls.signature, "signature")?);
    let message_hash = G2Projective::from(g2(&bls.message_hash, "message_hash")?);

    Ok(bls_verify(&signature, &[message_hash], &[public_key]))
}

/// Parses a G1 point from its uncompressed `x || y` encoding, rejecting one off
/// the curve or outside the prime-order subgroup.
fn g1(point: &G1, label: &'static str) -> Result<G1Affine, FilecoinError> {
    let mut bytes = [0u8; 96];
    pad_be(&mut bytes[..48], &point.x, label)?;
    pad_be(&mut bytes[48..], &point.y, label)?;
    Option::from(G1Affine::from_uncompressed(&bytes)).ok_or(FilecoinError::InvalidPoint(label))
}

/// Parses a G2 point into `blstrs`'s uncompressed encoding, which orders each
/// quadratic-extension coordinate imaginary-part-first (`c1` then `c0`), the
/// reverse of the vector's `[c0, c1]`. Rejects a point off the curve or outside
/// the prime-order subgroup.
fn g2(point: &G2, label: &'static str) -> Result<G2Affine, FilecoinError> {
    let mut bytes = [0u8; 192];
    pad_be(&mut bytes[..48], &point.x[1], label)?;
    pad_be(&mut bytes[48..96], &point.x[0], label)?;
    pad_be(&mut bytes[96..144], &point.y[1], label)?;
    pad_be(&mut bytes[144..], &point.y[0], label)?;
    Option::from(G2Affine::from_uncompressed(&bytes)).ok_or(FilecoinError::InvalidPoint(label))
}

/// Right-aligns an element's big-endian bytes into a 48-byte coordinate slot.
fn pad_be(slot: &mut [u8], element: &Element, label: &'static str) -> Result<(), FilecoinError> {
    let bytes = element.as_bytes();
    let start = slot
        .len()
        .checked_sub(bytes.len())
        .ok_or(FilecoinError::ElementTooWide(label))?;
    slot[start..].copy_from_slice(bytes);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SAMPLE_BLS_VECTOR;
    use crate::load::parse_vector;

    fn statement(vector: &mut Vector) -> &mut BlsSignature {
        match &mut vector.statement {
            Statement::BlsSignature(bls) => bls,
            Statement::Groth16(_) => panic!("bls sample should be a BLS signature statement"),
        }
    }

    #[test]
    fn accepts_the_committed_bls_signature() {
        let vector = parse_vector(SAMPLE_BLS_VECTOR).expect("bls sample should parse");
        assert!(
            verify(&vector).expect("adapter should run"),
            "the audited verifier must accept a valid signature"
        );
    }

    #[test]
    fn rejects_a_signature_against_the_wrong_message() {
        let mut vector = parse_vector(SAMPLE_BLS_VECTOR).expect("bls sample should parse");
        // Re-check the signature against a different valid G2 point (the signature
        // itself); a correct verifier must reject it.
        let bls = statement(&mut vector);
        bls.message_hash = bls.signature.clone();
        assert!(
            !verify(&vector).expect("adapter should run"),
            "the verifier must reject a signature over the wrong message"
        );
    }
}
