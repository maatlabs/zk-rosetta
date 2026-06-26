//! Data model, loader, and validator for the zk-rosetta catalog.
//!
//! The catalog is a set of TOML files under `data/<ecosystem>/<id>.toml`, each
//! deserialized into an [`Entry`]. The [`Entry`] type is the canonical
//! schema: the JSON Schema published for tooling is derived from it
//! (see [`schema_json`]), and correctness is enforced by strict deserialization
//! plus the invariant checks in [`validate()`].

mod drift;
mod load;
mod model;
mod validate;

pub use drift::{
    Fetched, Finding, ParseError, Source, Upstream, compare, resolve, source_for, sources,
};
pub use load::{LoadError, LoadedEntry, load_dir, parse_entry};
pub use model::{
    Category, Ecosystem, Entry, Implementation, Layer, Primitive, Relationships, SourceKind, Status,
};
pub use validate::{ValidationError, validate};

/// Returns the JSON Schema for an [`Entry`], derived from the Rust types.
pub fn schema_json() -> serde_json::Result<String> {
    serde_json::to_string_pretty(&schemars::schema_for!(Entry))
}
