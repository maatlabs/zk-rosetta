//! The catalog data model.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A single zero-knowledge-related protocol proposal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Proposal {
    /// Native identifier, e.g. `EIP-197`, `BIP-340`, or `SIMD-0129`.
    pub id: String,
    /// The proposal's official title.
    pub title: String,
    /// The ecosystem the proposal belongs to.
    pub ecosystem: Ecosystem,
    /// The layer at which the proposal operates.
    pub layer: Layer,
    /// The catalog's normalized category.
    pub category: Category,
    /// The normalized status, comparable across ecosystems.
    pub status: Status,
    /// The ecosystem's own status string, preserved verbatim.
    pub native_status: String,
    /// The cryptographic primitive the proposal exposes, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primitive: Option<Primitive>,
    /// One-line summary of what the proposal unlocks.
    pub enables: String,
    /// Canonical specification URL.
    pub spec: String,
    /// Known implementations of the proposal.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub implementations: Vec<Implementation>,
    /// Relationships to other proposals.
    #[serde(default, skip_serializing_if = "Relationships::is_empty")]
    pub relationships: Relationships,
    /// References used to write the entry.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<String>,
    /// Prose description of the proposal.
    pub notes: String,
}

/// The ecosystem a proposal belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Ecosystem {
    /// Ethereum and its standards (EIPs, ERCs).
    Ethereum,
    /// Bitcoin and its standards (BIPs).
    Bitcoin,
    /// Solana and its standards (SIMDs).
    Solana,
}

impl Ecosystem {
    /// The directory name used for this ecosystem under `data/`.
    pub fn dir(self) -> &'static str {
        match self {
            Self::Ethereum => "ethereum",
            Self::Bitcoin => "bitcoin",
            Self::Solana => "solana",
        }
    }
}

/// The layer at which a proposal operates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum Layer {
    /// Base-layer protocol change.
    #[serde(rename = "L1")]
    L1,
    /// Layer-two protocol change.
    #[serde(rename = "L2")]
    L2,
    /// Application or interface standard riding on a base layer.
    #[serde(rename = "app")]
    App,
}

/// The catalog's normalized proposal category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Category {
    /// A cryptographic primitive or curve operation.
    Primitive,
    /// An ABI or interface standard.
    Interface,
    /// Settlement, data-availability, or other supporting infrastructure.
    Infrastructure,
    /// An identity or nullifier standard.
    Identity,
    /// A privacy standard.
    Privacy,
    /// A proof-verification program or proof system.
    ProofSystem,
    /// A multi-signature scheme.
    MultiSig,
}

/// A proposal's normalized status, mapped from each ecosystem's native vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// Early concept, not yet a formal draft.
    Idea,
    /// Formal draft under active authorship.
    Draft,
    /// Under formal review or last call.
    Review,
    /// Accepted but not yet implemented.
    Accepted,
    /// Implemented, but not necessarily live on mainnet.
    Implemented,
    /// Finalized or live on mainnet.
    Final,
    /// Inactive draft that has stalled.
    Stagnant,
    /// Withdrawn by its authors.
    Withdrawn,
    /// Replaced by another proposal.
    Superseded,
}

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

/// A known implementation of a proposal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Implementation {
    /// Human-readable name of the implementation.
    pub name: String,
    /// Implementation language, e.g. `solidity` or `rust`.
    pub language: String,
    /// Source or documentation URL.
    pub url: String,
    /// Whether the implementation has been independently audited.
    pub audited: bool,
    /// Link to the audit report, when one exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audit_ref: Option<String>,
}

/// A proposal's relationships to other proposals.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Relationships {
    /// Proposals this one supersedes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supersedes: Vec<String>,
    /// Proposals that supersede this one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub superseded_by: Vec<String>,
    /// Proposals this one depends on.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    /// Proposals in other ecosystems that expose the same primitive.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub equivalent_to: Vec<String>,
}

impl Relationships {
    /// Returns `true` when no relationships are recorded.
    pub fn is_empty(&self) -> bool {
        self.supersedes.is_empty()
            && self.superseded_by.is_empty()
            && self.depends_on.is_empty()
            && self.equivalent_to.is_empty()
    }
}
