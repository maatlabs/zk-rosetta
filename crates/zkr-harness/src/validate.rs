//! Structural well-formedness of test vectors.
//!
//! These checks confirm a vector is shaped correctly before any adapter marshals
//! it into an audited verifier; they never verify the proof themselves.

use crate::model::{Element, G1, G2, Groth16, Primitive, Statement, Vector};

/// A single well-formedness problem found by [`validate`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VectorError {
    /// The verifying key's input basis does not match the number of public inputs.
    #[error(
        "verifying key has {found} IC point(s) but {public} public input(s) require {expected}"
    )]
    IcLength {
        /// IC points present in the verifying key.
        found: usize,
        /// IC points the public inputs require (`public + 1`).
        expected: usize,
        /// Number of public inputs.
        public: usize,
    },
    /// A field element is not the byte width the primitive requires.
    #[error(
        "{location}: expected a {expected}-byte {primitive} field element but found {found} byte(s)"
    )]
    ElementWidth {
        /// Where the element sits in the vector.
        location: String,
        /// The primitive the width is required by.
        primitive: String,
        /// The required byte width.
        expected: usize,
        /// The width actually found.
        found: usize,
    },
}

/// Validates a vector's structure, returning every problem found.
///
/// An empty result means the vector is well-formed.
pub fn validate(vector: &Vector) -> Vec<VectorError> {
    let errors = match &vector.statement {
        Statement::Groth16(groth16) => validate_groth16_ic(groth16),
        Statement::BlsSignature(_) => None,
    };
    errors.into_iter().chain(element_widths(vector)).collect()
}

fn validate_groth16_ic(groth16: &Groth16) -> Option<VectorError> {
    let expected = groth16.public_inputs.len().saturating_add(1);
    let found = groth16.vk.ic.len();
    (found != expected).then_some(VectorError::IcLength {
        found,
        expected,
        public: groth16.public_inputs.len(),
    })
}

fn element_widths(vector: &Vector) -> Vec<VectorError> {
    let Some(width) = element_width(vector.primitive) else {
        return Vec::new();
    };
    let primitive = zkr_core::label(vector.primitive);
    labeled_elements(vector)
        .into_iter()
        .filter(|(_, element)| element.as_bytes().len() != width)
        .map(|(location, element)| VectorError::ElementWidth {
            location,
            primitive: primitive.clone(),
            expected: width,
            found: element.as_bytes().len(),
        })
        .collect()
}

/// The required field-element byte width for primitives whose encoding is known.
/// Primitives without a known width are not width-checked.
fn element_width(primitive: Primitive) -> Option<usize> {
    match primitive {
        Primitive::Bn254 => Some(32),
        Primitive::Bls12381 => Some(48),
        _ => None,
    }
}

fn labeled_elements(vector: &Vector) -> Vec<(String, &Element)> {
    match &vector.statement {
        Statement::Groth16(groth16) => groth16_elements(groth16),
        Statement::BlsSignature(bls) => g1("public_key", &bls.public_key)
            .into_iter()
            .chain(g2("signature", &bls.signature))
            .chain(g2("message_hash", &bls.message_hash))
            .collect(),
    }
}

fn groth16_elements(groth16: &Groth16) -> Vec<(String, &Element)> {
    let public = groth16
        .public_inputs
        .iter()
        .enumerate()
        .map(|(i, element)| (format!("public_inputs[{i}]"), element));
    let ic = groth16
        .vk
        .ic
        .iter()
        .enumerate()
        .flat_map(|(i, point)| g1(&format!("vk.ic[{i}]"), point));

    public
        .chain(g1("vk.alpha_g1", &groth16.vk.alpha_g1))
        .chain(g2("vk.beta_g2", &groth16.vk.beta_g2))
        .chain(g2("vk.gamma_g2", &groth16.vk.gamma_g2))
        .chain(g2("vk.delta_g2", &groth16.vk.delta_g2))
        .chain(ic)
        .chain(g1("proof.a", &groth16.proof.a))
        .chain(g2("proof.b", &groth16.proof.b))
        .chain(g1("proof.c", &groth16.proof.c))
        .collect()
}

fn g1<'a>(label: &str, point: &'a G1) -> [(String, &'a Element); 2] {
    [
        (format!("{label}.x"), &point.x),
        (format!("{label}.y"), &point.y),
    ]
}

fn g2<'a>(label: &str, point: &'a G2) -> [(String, &'a Element); 4] {
    [
        (format!("{label}.x[0]"), &point.x[0]),
        (format!("{label}.x[1]"), &point.x[1]),
        (format!("{label}.y[0]"), &point.y[0]),
        (format!("{label}.y[1]"), &point.y[1]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load::parse_vector;
    use crate::{SAMPLE_BLS_VECTOR, SAMPLE_VECTOR};

    fn sample() -> Vector {
        parse_vector(SAMPLE_VECTOR).expect("sample vector should parse")
    }

    fn groth16(vector: &mut Vector) -> &mut Groth16 {
        match &mut vector.statement {
            Statement::Groth16(groth16) => groth16,
            Statement::BlsSignature(_) => panic!("sample vector should be a Groth16 statement"),
        }
    }

    #[test]
    fn accepts_a_wellformed_vector() {
        assert!(validate(&sample()).is_empty());
    }

    #[test]
    fn accepts_a_wellformed_bls_vector() {
        let vector = parse_vector(SAMPLE_BLS_VECTOR).expect("bls sample should parse");
        assert!(validate(&vector).is_empty());
    }

    #[test]
    fn detects_ic_length_mismatch() {
        let mut vector = sample();
        let groth16 = groth16(&mut vector);
        groth16.public_inputs.push(groth16.public_inputs[0].clone());
        let errors = validate(&vector);
        assert!(errors.contains(&VectorError::IcLength {
            found: 2,
            expected: 3,
            public: 2,
        }));
    }

    #[test]
    fn detects_wrong_element_width() {
        let toml = SAMPLE_VECTOR.replace(
            "0x0000000000000000000000000000000000000000000000000000000000000001",
            "0x00000000000000000000000000000000000000000000000000000000000001",
        );
        let vector = parse_vector(&toml).expect("shorter element still parses");
        let errors = validate(&vector);
        assert!(errors.iter().any(|error| matches!(
            error,
            VectorError::ElementWidth {
                found: 31,
                expected: 32,
                ..
            }
        )));
    }

    #[test]
    fn detects_wrong_bls_element_width() {
        // Drop a byte from the public key's x-coordinate: a 47-byte BLS12-381
        // element a correct vector never carries.
        let toml = SAMPLE_BLS_VECTOR.replace(
            "0x0498474b8f74ec0b027cfc31ac5773f2655382d5ac6081a91e122ac812ac25b1cd8680827c43bc4bedd38e1f5d1cff05",
            "0x98474b8f74ec0b027cfc31ac5773f2655382d5ac6081a91e122ac812ac25b1cd8680827c43bc4bedd38e1f5d1cff05",
        );
        let vector = parse_vector(&toml).expect("shorter element still parses");
        let errors = validate(&vector);
        assert!(errors.iter().any(|error| matches!(
            error,
            VectorError::ElementWidth {
                found: 47,
                expected: 48,
                ..
            }
        )));
    }
}
