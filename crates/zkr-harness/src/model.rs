//! The test-vector data model.

use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize, Serializer};
pub use zkr_core::Primitive;

/// One ecosystem-neutral test vector: a primitive, the verdict a correct
/// verifier must reach, and the statement that verifier checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Vector {
    /// The cryptographic primitive, named as in the catalog's taxonomy.
    pub primitive: Primitive,
    /// Whether a correct verifier must accept or reject the vector.
    pub expected: Expected,
    /// The statement an audited verifier checks, tagged by its kind.
    pub statement: Statement,
}

/// The statement a vector pins, in a shape specific to its proof system.
///
/// The externally tagged form keys the statement's fields under the system name
/// (`[statement.groth16]`, `[statement.bls-signature]`), so each adapter matches
/// the variant it understands and the loader rejects an unknown system.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Statement {
    /// A Groth16 zk-SNARK: public inputs verified against a key and a proof.
    Groth16(Box<Groth16>),
    /// A BLS signature, verified as the pairing relation `e(g1, signature) ==
    /// e(public_key, message_hash)`.
    BlsSignature(Box<BlsSignature>),
}

/// A Groth16 zk-SNARK statement: public inputs, the verifying key, and the proof.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Groth16 {
    /// The public signals, in statement order.
    pub public_inputs: Vec<Element>,
    /// The verifying key.
    pub vk: VerifyingKey,
    /// The proof.
    pub proof: Proof,
}

/// A BLS signature statement over BLS12-381, with the message already hashed to
/// the curve so adapters verify the identical pairing relation on identical
/// bytes, never hashing the message themselves.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlsSignature {
    /// The public key in G1, `pk = sk * g1`.
    pub public_key: G1,
    /// The signature in G2, `sig = sk * H(m)`.
    pub signature: G2,
    /// The hashed message `H(m)` in G2.
    pub message_hash: G2,
}

/// Whether a correct verifier must accept or reject the vector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Expected {
    /// A correct verifier accepts the proof.
    Accept,
    /// A correct verifier rejects the proof.
    Reject,
}

/// A field element or point coordinate, held as its big-endian bytes.
///
/// In `vector.toml` an element is written as `0x`-prefixed hex; deserialization
/// rejects any value lacking the prefix or containing non-hex digits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element(Vec<u8>);

impl Element {
    /// The big-endian bytes of the element.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl Serialize for Element {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("0x{}", hex::encode(&self.0)))
    }
}

impl<'de> Deserialize<'de> for Element {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        let digits = raw.strip_prefix("0x").ok_or_else(|| {
            de::Error::custom(format!("field element must be 0x-prefixed: `{raw}`"))
        })?;
        hex::decode(digits)
            .map(Element)
            .map_err(|err| de::Error::custom(format!("invalid hex field element `{raw}`: {err}")))
    }
}

/// A Groth16 verifying key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifyingKey {
    /// The `alpha` element in G1.
    pub alpha_g1: G1,
    /// The `beta` element in G2.
    pub beta_g2: G2,
    /// The `gamma` element in G2.
    pub gamma_g2: G2,
    /// The `delta` element in G2.
    pub delta_g2: G2,
    /// The input commitment basis: one point per public input, plus one.
    pub ic: Vec<G1>,
}

/// A Groth16 proof.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Proof {
    /// The `A` element in G1.
    pub a: G1,
    /// The `B` element in G2.
    pub b: G2,
    /// The `C` element in G1.
    pub c: G1,
}

/// An affine point on the curve's base group G1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct G1 {
    /// Affine x-coordinate.
    pub x: Element,
    /// Affine y-coordinate.
    pub y: Element,
}

/// An affine point on the curve's twist group G2.
///
/// Each coordinate is an element of the quadratic extension field written
/// `[c0, c1]`, representing `c0 + c1 * u`---the mathematical coordinate order.
/// Adapters apply any ecosystem-specific reordering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct G2 {
    /// Extension-field x-coordinate as `[c0, c1]`.
    pub x: [Element; 2],
    /// Extension-field y-coordinate as `[c0, c1]`.
    pub y: [Element; 2],
}
