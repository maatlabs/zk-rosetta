//! The cross-ecosystem cryptographic-primitive taxonomy.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A cryptographic primitive exposed by a catalog entry; powers cross-ecosystem grouping.
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

impl Primitive {
    /// Whether the parity harness can, in principle, prove bit-identical
    /// verdicts for this primitive across ecosystems.
    ///
    /// True only for a single fixed function or relation with a shared
    /// on-the-wire encoding: the pairing-friendly curves [`Self::Bn254`] and
    /// [`Self::Bls12381`] and the signature curves [`Self::Secp256r1`] and
    /// [`Self::Secp256k1`]. Parameterized constructions (such as
    /// [`Self::Poseidon`], instantiated over a different field per chain) and
    /// setup-bound ones ([`Self::Kzg`]) are conceptually equivalent across
    /// ecosystems yet are different functions that cannot agree on the same
    /// bytes, so they are excluded.
    pub fn parity_provable(self) -> bool {
        matches!(
            self,
            Self::Bn254 | Self::Bls12381 | Self::Secp256r1 | Self::Secp256k1
        )
    }
}
