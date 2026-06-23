//! Data model, loader, and validator for the zk-rosetta proposal catalog.
//!
//! The catalog is a set of TOML files under `data/<ecosystem>/<id>.toml`, each
//! deserialized into a [`Proposal`]. The [`Proposal`] type is the canonical
//! schema: the JSON Schema published for tooling is derived from it
//! (see [`schema_json`]), and correctness is enforced by strict deserialization
//! plus the invariant checks in [`validate()`].

mod load;
mod model;
mod validate;

pub use load::{LoadError, LoadedProposal, load_dir, parse_proposal};
pub use model::{
    Category, Ecosystem, Implementation, Layer, Primitive, Proposal, Relationships, Status,
};
pub use validate::{ValidationError, validate};

/// Returns the JSON Schema for a [`Proposal`], derived from the Rust types.
pub fn schema_json() -> serde_json::Result<String> {
    serde_json::to_string_pretty(&schemars::schema_for!(Proposal))
}
