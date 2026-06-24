//! Every vector committed under `vectors/` must load and pass well-formedness.

use std::path::Path;

use zkr_harness::{Expected, LoadedVector, Primitive, ProofSystem, load_dir, validate};

fn committed() -> Vec<LoadedVector> {
    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../vectors"));
    load_dir(root).expect("the committed vectors directory should load")
}

#[test]
fn every_committed_vector_is_wellformed() {
    let vectors = committed();
    assert!(
        !vectors.is_empty(),
        "expected at least one committed vector"
    );
    for loaded in &vectors {
        let errors = validate(&loaded.value);
        assert!(
            errors.is_empty(),
            "{} is malformed: {errors:?}",
            loaded.path.display()
        );
    }
}

#[test]
fn bn254_groth16_multiplier_proves_three_times_eleven() {
    let loaded = committed()
        .into_iter()
        .find(|loaded| {
            loaded
                .path
                .to_string_lossy()
                .contains("bn254-groth16-multiplier")
        })
        .expect("the bn254-groth16-multiplier vector should be present");
    let vector = loaded.value;

    assert_eq!(vector.proof_system, ProofSystem::Groth16);
    assert_eq!(vector.primitive, Primitive::Bn254);
    assert_eq!(vector.expected, Expected::Accept);
    assert_eq!(vector.vk.ic.len(), vector.public_inputs.len() + 1);

    // The statement's only public signal is the product c = 3 * 11 = 33.
    let mut expected = [0u8; 32];
    expected[31] = 33;
    assert_eq!(vector.public_inputs.len(), 1);
    assert_eq!(vector.public_inputs[0].as_bytes(), expected);
}
