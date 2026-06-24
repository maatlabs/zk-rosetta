//! Display labels derived from serde wire forms.

use serde::Serialize;

/// The serde wire form of `value`, reused as its canonical display label.
///
/// Enums in this workspace carry `#[serde(rename = ...)]` attributes that fix
/// their names (for example `BN254`); rendering through serde keeps
/// every display site---filter controls, comparison groupings, validation
/// messages---in lockstep with the wire form instead of duplicating the strings.
/// A value whose serde form is not a string yields the empty string.
pub fn label<T: Serialize>(value: T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Primitive;

    #[test]
    fn label_tracks_the_serde_rename_not_the_variant_name() {
        // The label must follow the serde rename, never the Rust identifier, so
        // primitives surface as `BN254`/`BLS12-381`, not `Bn254`/`Bls12381`.
        assert_eq!(label(Primitive::Bn254), "BN254");
        assert_eq!(label(Primitive::Bls12381), "BLS12-381");
    }
}
