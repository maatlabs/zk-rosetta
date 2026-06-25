//! The cross-ecosystem cryptographic-primitive taxonomy.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A cryptographic primitive exposed by a proposal; powers cross-ecosystem grouping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum Primitive {
    /// BN254 / alt_bn128 pairing-friendly curve.
    #[serde(rename = "BN254")]
    Bn254,
    /// BLS12-381 pairing-friendly curve.
    #[serde(rename = "BLS12-381")]
    Bls12381,
    /// secp256r1 (NIST P-256) curve.
    #[serde(rename = "secp256r1")]
    Secp256r1,
    /// secp256k1 curve.
    #[serde(rename = "secp256k1")]
    Secp256k1,
    /// Curve25519.
    #[serde(rename = "curve25519")]
    Curve25519,
    /// Jubjub, the curve embedded in BLS12-381's scalar field (Zcash Sapling).
    #[serde(rename = "Jubjub")]
    Jubjub,
    /// Poseidon hash.
    #[serde(rename = "Poseidon")]
    Poseidon,
    /// KZG polynomial commitments.
    #[serde(rename = "KZG")]
    Kzg,
    /// BLAKE2 hash.
    #[serde(rename = "Blake2")]
    Blake2,
}
