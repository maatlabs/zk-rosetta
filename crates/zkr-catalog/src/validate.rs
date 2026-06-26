//! Catalog invariants.

use std::collections::HashMap;

use url::Url;

use crate::load::LoadedEntry;
use crate::model::{Entry, Primitive};

/// A single catalog problem found by [`validate`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ValidationError {
    /// The same id appears in more than one file.
    #[error("duplicate id `{id}` (files: {paths})")]
    DuplicateId {
        /// The repeated id.
        id: String,
        /// Comma-separated paths that share the id.
        paths: String,
    },
    /// A file's name does not match its id.
    #[error("`{path}`: filename does not match id `{id}` (expected `{expected}.toml`)")]
    FilenameMismatch {
        /// The offending file.
        path: String,
        /// The id declared in the file.
        id: String,
        /// The expected lowercase file stem.
        expected: String,
    },
    /// A file lives under a directory that does not match its ecosystem.
    #[error("`{path}`: id `{id}` is under `{found}/` but its ecosystem is `{expected}`")]
    EcosystemDirMismatch {
        /// The offending file.
        path: String,
        /// The id declared in the file.
        id: String,
        /// The directory the file was found in.
        found: String,
        /// The directory implied by the ecosystem.
        expected: String,
    },
    /// A relationship references an id that is not in the catalog.
    #[error("`{id}`: {field} references unknown id `{referenced}`")]
    DanglingReference {
        /// The entry holding the relationship.
        id: String,
        /// The relationship field.
        field: String,
        /// The referenced id.
        referenced: String,
    },
    /// A relationship references its own entry.
    #[error("`{id}`: {field} references itself")]
    SelfReference {
        /// The entry holding the relationship.
        id: String,
        /// The relationship field.
        field: String,
    },
    /// A URL is missing or not a well-formed `http`/`https` URL.
    #[error("`{id}`: malformed {field} URL `{value}`")]
    MalformedUrl {
        /// The entry holding the URL.
        id: String,
        /// The field the URL came from.
        field: String,
        /// The offending value.
        value: String,
    },
    /// `a` declares `b` equivalent but `b` does not declare `a`.
    #[error("equivalence between `{a}` and `{b}` is not symmetric")]
    AsymmetricEquivalence {
        /// The entry declaring the equivalence.
        a: String,
        /// The entry that fails to reciprocate.
        b: String,
    },
    /// A supersession edge is not mirrored by its inverse.
    #[error("supersession of `{superseded}` by `{superseder}` is not mirrored")]
    InconsistentSupersession {
        /// The superseding entry.
        superseder: String,
        /// The superseded entry.
        superseded: String,
    },
    /// A `proven_by` entry names a vector absent from `vectors/`.
    #[error("`{id}`: proven_by references unknown vector `{vector}`")]
    DanglingVector {
        /// The entry holding the reference.
        id: String,
        /// The referenced vector name.
        vector: String,
    },
    /// A `proven_by` vector exercises a different primitive than the entry.
    #[error(
        "`{id}`: proven_by vector `{vector}` exercises {vector_primitive} but the entry's primitive is {entry_primitive}"
    )]
    VectorPrimitiveMismatch {
        /// The entry holding the reference.
        id: String,
        /// The referenced vector name.
        vector: String,
        /// The entry's declared primitive, or `unset` when absent.
        entry_primitive: String,
        /// The primitive the referenced vector exercises.
        vector_primitive: String,
    },
}

/// Validates a loaded catalog against the committed vectors, returning every
/// problem found.
///
/// `vectors` maps each `vectors/<name>` directory to the primitive its committed
/// vector exercises; it is the authority every `proven_by` reference is checked
/// against. An empty result means the catalog is valid.
pub fn validate(
    loaded: &[LoadedEntry],
    vectors: &HashMap<String, Primitive>,
) -> Vec<ValidationError> {
    let index = loaded
        .iter()
        .map(|entry| (entry.value.id.as_str(), &entry.value))
        .collect::<HashMap<&str, &Entry>>();

    duplicate_ids(loaded)
        .into_iter()
        .chain(loaded.iter().flat_map(path_consistency))
        .chain(
            loaded
                .iter()
                .flat_map(|entry| url_wellformedness(&entry.value)),
        )
        .chain(
            loaded
                .iter()
                .flat_map(|entry| referential_integrity(&entry.value, &index)),
        )
        .chain(
            loaded
                .iter()
                .flat_map(|entry| edge_symmetry(&entry.value, &index)),
        )
        .chain(
            loaded
                .iter()
                .flat_map(|entry| proven_by_integrity(&entry.value, vectors)),
        )
        .collect()
}

fn duplicate_ids(loaded: &[LoadedEntry]) -> Vec<ValidationError> {
    let mut by_id: HashMap<&str, Vec<String>> = HashMap::new();
    for entry in loaded {
        by_id
            .entry(entry.value.id.as_str())
            .or_default()
            .push(entry.path.display().to_string());
    }
    by_id
        .into_iter()
        .filter(|(_, paths)| paths.len() > 1)
        .map(|(id, mut paths)| {
            paths.sort();
            ValidationError::DuplicateId {
                id: id.to_string(),
                paths: paths.join(", "),
            }
        })
        .collect()
}

fn path_consistency(loaded: &LoadedEntry) -> Vec<ValidationError> {
    let entry = &loaded.value;
    let path = loaded.path.display().to_string();
    let mut errors = Vec::new();

    let expected_stem = entry.id.to_ascii_lowercase();
    let stem = loaded
        .path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    if stem != expected_stem {
        errors.push(ValidationError::FilenameMismatch {
            path: path.clone(),
            id: entry.id.clone(),
            expected: expected_stem,
        });
    }

    let found = loaded
        .path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    let expected = entry.ecosystem.dir();
    if found != expected {
        errors.push(ValidationError::EcosystemDirMismatch {
            path,
            id: entry.id.clone(),
            found: found.to_string(),
            expected: expected.to_string(),
        });
    }

    errors
}

fn url_wellformedness(entry: &Entry) -> Vec<ValidationError> {
    let mut fields: Vec<(String, &str)> = vec![("spec".to_string(), entry.spec.as_str())];
    fields.extend(
        entry
            .sources
            .iter()
            .enumerate()
            .map(|(i, source)| (format!("sources[{i}]"), source.as_str())),
    );
    for (i, implementation) in entry.implementations.iter().enumerate() {
        fields.push((
            format!("implementations[{i}].url"),
            implementation.url.as_str(),
        ));
        if let Some(audit) = &implementation.audit_ref {
            fields.push((format!("implementations[{i}].audit_ref"), audit.as_str()));
        }
    }

    fields
        .into_iter()
        .filter(|(_, value)| !is_http_url(value))
        .map(|(field, value)| ValidationError::MalformedUrl {
            id: entry.id.clone(),
            field,
            value: value.to_string(),
        })
        .collect()
}

fn is_http_url(value: &str) -> bool {
    Url::parse(value)
        .map(|url| matches!(url.scheme(), "http" | "https"))
        .unwrap_or(false)
}

fn referential_integrity(entry: &Entry, index: &HashMap<&str, &Entry>) -> Vec<ValidationError> {
    let relationships = &entry.relationships;
    [
        ("supersedes", &relationships.supersedes),
        ("superseded_by", &relationships.superseded_by),
        ("depends_on", &relationships.depends_on),
        ("equivalent_to", &relationships.equivalent_to),
    ]
    .into_iter()
    .flat_map(|(field, ids)| {
        ids.iter().filter_map(move |referenced| {
            if referenced == &entry.id {
                Some(ValidationError::SelfReference {
                    id: entry.id.clone(),
                    field: field.to_string(),
                })
            } else if !index.contains_key(referenced.as_str()) {
                Some(ValidationError::DanglingReference {
                    id: entry.id.clone(),
                    field: field.to_string(),
                    referenced: referenced.clone(),
                })
            } else {
                None
            }
        })
    })
    .collect()
}

fn edge_symmetry(entry: &Entry, index: &HashMap<&str, &Entry>) -> Vec<ValidationError> {
    let relationships = &entry.relationships;
    let mut errors = Vec::new();

    for other_id in &relationships.equivalent_to {
        if let Some(other) = index.get(other_id.as_str())
            && !other.relationships.equivalent_to.contains(&entry.id)
        {
            errors.push(ValidationError::AsymmetricEquivalence {
                a: entry.id.clone(),
                b: other_id.clone(),
            });
        }
    }

    for other_id in &relationships.superseded_by {
        if let Some(other) = index.get(other_id.as_str())
            && !other.relationships.supersedes.contains(&entry.id)
        {
            errors.push(ValidationError::InconsistentSupersession {
                superseder: other_id.clone(),
                superseded: entry.id.clone(),
            });
        }
    }

    for other_id in &relationships.supersedes {
        if let Some(other) = index.get(other_id.as_str())
            && !other.relationships.superseded_by.contains(&entry.id)
        {
            errors.push(ValidationError::InconsistentSupersession {
                superseder: entry.id.clone(),
                superseded: other_id.clone(),
            });
        }
    }

    errors
}

fn proven_by_integrity(
    entry: &Entry,
    vectors: &HashMap<String, Primitive>,
) -> Vec<ValidationError> {
    entry
        .proven_by
        .iter()
        .filter_map(|vector| match vectors.get(vector) {
            None => Some(ValidationError::DanglingVector {
                id: entry.id.clone(),
                vector: vector.clone(),
            }),
            Some(found) if Some(*found) != entry.primitive => {
                Some(ValidationError::VectorPrimitiveMismatch {
                    id: entry.id.clone(),
                    vector: vector.clone(),
                    entry_primitive: entry
                        .primitive
                        .map(zkr_core::label)
                        .unwrap_or_else(|| "unset".to_string()),
                    vector_primitive: zkr_core::label(*found),
                })
            }
            Some(_) => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Category, Ecosystem, Layer, Relationships, SourceKind, Status};

    fn entry(id: &str) -> Entry {
        Entry {
            id: id.to_string(),
            title: "Title".to_string(),
            ecosystem: Ecosystem::Ethereum,
            kind: SourceKind::Proposal,
            layer: Layer::L1,
            category: Category::Primitive,
            status: Status::Final,
            native_status: "Final".to_string(),
            primitive: None,
            enables: "enables".to_string(),
            spec: "https://example.com/spec".to_string(),
            implementations: Vec::new(),
            relationships: Relationships::default(),
            proven_by: Vec::new(),
            sources: Vec::new(),
            notes: "notes".to_string(),
        }
    }

    fn loaded(entry: Entry) -> LoadedEntry {
        let path = format!("data/ethereum/{}.toml", entry.id.to_ascii_lowercase());
        LoadedEntry {
            path: path.into(),
            value: entry,
        }
    }

    fn vectors(entries: &[(&str, Primitive)]) -> HashMap<String, Primitive> {
        entries
            .iter()
            .map(|(name, primitive)| (name.to_string(), *primitive))
            .collect()
    }

    #[test]
    fn accepts_a_consistent_catalog() {
        let mut a = entry("EIP-1");
        let mut b = entry("EIP-2");
        a.relationships.equivalent_to = vec!["EIP-2".to_string()];
        b.relationships.equivalent_to = vec!["EIP-1".to_string()];
        let errors = validate(&[loaded(a), loaded(b)], &vectors(&[]));
        assert!(errors.is_empty(), "expected no errors, got {errors:?}");
    }

    #[test]
    fn detects_duplicate_ids() {
        let errors = validate(
            &[loaded(entry("EIP-1")), loaded(entry("EIP-1"))],
            &vectors(&[]),
        );
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ValidationError::DuplicateId { id, .. } if id == "EIP-1"))
        );
    }

    #[test]
    fn detects_dangling_reference() {
        let mut a = entry("EIP-1");
        a.relationships.depends_on = vec!["EIP-999".to_string()];
        let errors = validate(&[loaded(a)], &vectors(&[]));
        assert!(errors.contains(&ValidationError::DanglingReference {
            id: "EIP-1".to_string(),
            field: "depends_on".to_string(),
            referenced: "EIP-999".to_string(),
        }));
    }

    #[test]
    fn detects_self_reference() {
        let mut a = entry("EIP-1");
        a.relationships.depends_on = vec!["EIP-1".to_string()];
        let errors = validate(&[loaded(a)], &vectors(&[]));
        assert!(errors.contains(&ValidationError::SelfReference {
            id: "EIP-1".to_string(),
            field: "depends_on".to_string(),
        }));
    }

    #[test]
    fn detects_asymmetric_equivalence() {
        let mut a = entry("EIP-1");
        let b = entry("EIP-2");
        a.relationships.equivalent_to = vec!["EIP-2".to_string()];
        let errors = validate(&[loaded(a), loaded(b)], &vectors(&[]));
        assert!(errors.contains(&ValidationError::AsymmetricEquivalence {
            a: "EIP-1".to_string(),
            b: "EIP-2".to_string(),
        }));
    }

    #[test]
    fn detects_inconsistent_supersession() {
        let mut a = entry("EIP-1");
        let b = entry("EIP-2");
        a.relationships.superseded_by = vec!["EIP-2".to_string()];
        let errors = validate(&[loaded(a), loaded(b)], &vectors(&[]));
        assert!(errors.contains(&ValidationError::InconsistentSupersession {
            superseder: "EIP-2".to_string(),
            superseded: "EIP-1".to_string(),
        }));
    }

    #[test]
    fn detects_malformed_url() {
        let mut a = entry("EIP-1");
        a.spec = "not-a-url".to_string();
        let errors = validate(&[loaded(a)], &vectors(&[]));
        assert!(
            errors.iter().any(
                |e| matches!(e, ValidationError::MalformedUrl { field, .. } if field == "spec")
            )
        );
    }

    #[test]
    fn detects_filename_mismatch() {
        let wrong = LoadedEntry {
            path: "data/ethereum/wrong.toml".into(),
            value: entry("EIP-1"),
        };
        let errors = validate(&[wrong], &vectors(&[]));
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ValidationError::FilenameMismatch { .. }))
        );
    }

    #[test]
    fn detects_dangling_vector_reference() {
        let mut a = entry("EIP-1");
        a.primitive = Some(Primitive::Bn254);
        a.proven_by = vec!["no-such-vector".to_string()];
        let errors = validate(&[loaded(a)], &vectors(&[("real-vector", Primitive::Bn254)]));
        assert!(errors.contains(&ValidationError::DanglingVector {
            id: "EIP-1".to_string(),
            vector: "no-such-vector".to_string(),
        }));
    }

    #[test]
    fn detects_vector_primitive_mismatch() {
        let mut a = entry("EIP-1");
        a.primitive = Some(Primitive::Bn254);
        a.proven_by = vec!["bls-vector".to_string()];
        let errors = validate(
            &[loaded(a)],
            &vectors(&[("bls-vector", Primitive::Bls12381)]),
        );
        assert!(errors.contains(&ValidationError::VectorPrimitiveMismatch {
            id: "EIP-1".to_string(),
            vector: "bls-vector".to_string(),
            entry_primitive: "BN254".to_string(),
            vector_primitive: "BLS12-381".to_string(),
        }));
    }

    #[test]
    fn accepts_a_proven_by_reference_with_matching_primitive() {
        let mut a = entry("EIP-1");
        a.primitive = Some(Primitive::Bn254);
        a.proven_by = vec!["bn254-vector".to_string()];
        let errors = validate(
            &[loaded(a)],
            &vectors(&[("bn254-vector", Primitive::Bn254)]),
        );
        assert!(errors.is_empty(), "expected no errors, got {errors:?}");
    }
}
