//! Shared infrastructure for the zk-rosetta workspace.
//!
//! This crate holds the pieces several crates need but none should own: the
//! cross-ecosystem [`Primitive`] taxonomy, the TOML loading layer ([`LoadError`],
//! [`read_sorted`], the generic [`load_file`], and [`Loaded`]), and the [`label()`]
//! helper that turns an enum's serde wire form into its canonical display string.
//! It authors no cryptography and knows nothing of any consuming crate's domain
//! types.

mod label;
mod load;
mod primitive;

pub use label::label;
pub use load::{LoadError, Loaded, load_file, read_sorted};
pub use primitive::Primitive;
