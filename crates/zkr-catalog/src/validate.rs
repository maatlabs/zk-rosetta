//! Catalog invariants.

use std::collections::HashMap;

use url::Url;

use crate::load::LoadedProposal;
use crate::model::Proposal;

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
        /// The proposal holding the relationship.
        id: String,
        /// The relationship field.
        field: String,
        /// The referenced id.
        referenced: String,
    },
    /// A relationship references its own proposal.
    #[error("`{id}`: {field} references itself")]
    SelfReference {
        /// The proposal holding the relationship.
        id: String,
        /// The relationship field.
        field: String,
    },
    /// A URL is missing or not a well-formed `http`/`https` URL.
    #[error("`{id}`: malformed {field} URL `{value}`")]
    MalformedUrl {
        /// The proposal holding the URL.
        id: String,
        /// The field the URL came from.
        field: String,
        /// The offending value.
        value: String,
    },
    /// `a` declares `b` equivalent but `b` does not declare `a`.
    #[error("equivalence between `{a}` and `{b}` is not symmetric")]
    AsymmetricEquivalence {
        /// The proposal declaring the equivalence.
        a: String,
        /// The proposal that fails to reciprocate.
        b: String,
    },
    /// A supersession edge is not mirrored by its inverse.
    #[error("supersession of `{superseded}` by `{superseder}` is not mirrored")]
    InconsistentSupersession {
        /// The superseding proposal.
        superseder: String,
        /// The superseded proposal.
        superseded: String,
    },
}

/// Validates a loaded catalog, returning every problem found.
///
/// An empty result means the catalog is valid.
pub fn validate(loaded: &[LoadedProposal]) -> Vec<ValidationError> {
    let index = loaded
        .iter()
        .map(|entry| (entry.value.id.as_str(), &entry.value))
        .collect::<HashMap<&str, &Proposal>>();

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
        .collect()
}

fn duplicate_ids(loaded: &[LoadedProposal]) -> Vec<ValidationError> {
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

fn path_consistency(entry: &LoadedProposal) -> Vec<ValidationError> {
    let proposal = &entry.value;
    let path = entry.path.display().to_string();
    let mut errors = Vec::new();

    let expected_stem = proposal.id.to_ascii_lowercase();
    let stem = entry
        .path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    if stem != expected_stem {
        errors.push(ValidationError::FilenameMismatch {
            path: path.clone(),
            id: proposal.id.clone(),
            expected: expected_stem,
        });
    }

    let found = entry
        .path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    let expected = proposal.ecosystem.dir();
    if found != expected {
        errors.push(ValidationError::EcosystemDirMismatch {
            path,
            id: proposal.id.clone(),
            found: found.to_string(),
            expected: expected.to_string(),
        });
    }

    errors
}

fn url_wellformedness(proposal: &Proposal) -> Vec<ValidationError> {
    let mut fields: Vec<(String, &str)> = vec![("spec".to_string(), proposal.spec.as_str())];
    fields.extend(
        proposal
            .sources
            .iter()
            .enumerate()
            .map(|(i, source)| (format!("sources[{i}]"), source.as_str())),
    );
    for (i, implementation) in proposal.implementations.iter().enumerate() {
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
            id: proposal.id.clone(),
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

fn referential_integrity(
    proposal: &Proposal,
    index: &HashMap<&str, &Proposal>,
) -> Vec<ValidationError> {
    let relationships = &proposal.relationships;
    [
        ("supersedes", &relationships.supersedes),
        ("superseded_by", &relationships.superseded_by),
        ("depends_on", &relationships.depends_on),
        ("equivalent_to", &relationships.equivalent_to),
    ]
    .into_iter()
    .flat_map(|(field, ids)| {
        ids.iter().filter_map(move |referenced| {
            if referenced == &proposal.id {
                Some(ValidationError::SelfReference {
                    id: proposal.id.clone(),
                    field: field.to_string(),
                })
            } else if !index.contains_key(referenced.as_str()) {
                Some(ValidationError::DanglingReference {
                    id: proposal.id.clone(),
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

fn edge_symmetry(proposal: &Proposal, index: &HashMap<&str, &Proposal>) -> Vec<ValidationError> {
    let relationships = &proposal.relationships;
    let mut errors = Vec::new();

    for other_id in &relationships.equivalent_to {
        if let Some(other) = index.get(other_id.as_str())
            && !other.relationships.equivalent_to.contains(&proposal.id)
        {
            errors.push(ValidationError::AsymmetricEquivalence {
                a: proposal.id.clone(),
                b: other_id.clone(),
            });
        }
    }

    for other_id in &relationships.superseded_by {
        if let Some(other) = index.get(other_id.as_str())
            && !other.relationships.supersedes.contains(&proposal.id)
        {
            errors.push(ValidationError::InconsistentSupersession {
                superseder: other_id.clone(),
                superseded: proposal.id.clone(),
            });
        }
    }

    for other_id in &relationships.supersedes {
        if let Some(other) = index.get(other_id.as_str())
            && !other.relationships.superseded_by.contains(&proposal.id)
        {
            errors.push(ValidationError::InconsistentSupersession {
                superseder: proposal.id.clone(),
                superseded: other_id.clone(),
            });
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Category, Ecosystem, Layer, Relationships, Status};

    fn proposal(id: &str) -> Proposal {
        Proposal {
            id: id.to_string(),
            title: "Title".to_string(),
            ecosystem: Ecosystem::Ethereum,
            layer: Layer::L1,
            category: Category::Primitive,
            status: Status::Final,
            native_status: "Final".to_string(),
            primitive: None,
            enables: "enables".to_string(),
            spec: "https://example.com/spec".to_string(),
            implementations: Vec::new(),
            relationships: Relationships::default(),
            sources: Vec::new(),
            notes: "notes".to_string(),
        }
    }

    fn loaded(proposal: Proposal) -> LoadedProposal {
        let path = format!("data/ethereum/{}.toml", proposal.id.to_ascii_lowercase());
        LoadedProposal {
            path: path.into(),
            value: proposal,
        }
    }

    #[test]
    fn accepts_a_consistent_catalog() {
        let mut a = proposal("EIP-1");
        let mut b = proposal("EIP-2");
        a.relationships.equivalent_to = vec!["EIP-2".to_string()];
        b.relationships.equivalent_to = vec!["EIP-1".to_string()];
        let errors = validate(&[loaded(a), loaded(b)]);
        assert!(errors.is_empty(), "expected no errors, got {errors:?}");
    }

    #[test]
    fn detects_duplicate_ids() {
        let errors = validate(&[loaded(proposal("EIP-1")), loaded(proposal("EIP-1"))]);
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ValidationError::DuplicateId { id, .. } if id == "EIP-1"))
        );
    }

    #[test]
    fn detects_dangling_reference() {
        let mut a = proposal("EIP-1");
        a.relationships.depends_on = vec!["EIP-999".to_string()];
        let errors = validate(&[loaded(a)]);
        assert!(errors.contains(&ValidationError::DanglingReference {
            id: "EIP-1".to_string(),
            field: "depends_on".to_string(),
            referenced: "EIP-999".to_string(),
        }));
    }

    #[test]
    fn detects_self_reference() {
        let mut a = proposal("EIP-1");
        a.relationships.depends_on = vec!["EIP-1".to_string()];
        let errors = validate(&[loaded(a)]);
        assert!(errors.contains(&ValidationError::SelfReference {
            id: "EIP-1".to_string(),
            field: "depends_on".to_string(),
        }));
    }

    #[test]
    fn detects_asymmetric_equivalence() {
        let mut a = proposal("EIP-1");
        let b = proposal("EIP-2");
        a.relationships.equivalent_to = vec!["EIP-2".to_string()];
        let errors = validate(&[loaded(a), loaded(b)]);
        assert!(errors.contains(&ValidationError::AsymmetricEquivalence {
            a: "EIP-1".to_string(),
            b: "EIP-2".to_string(),
        }));
    }

    #[test]
    fn detects_inconsistent_supersession() {
        let mut a = proposal("EIP-1");
        let b = proposal("EIP-2");
        a.relationships.superseded_by = vec!["EIP-2".to_string()];
        let errors = validate(&[loaded(a), loaded(b)]);
        assert!(errors.contains(&ValidationError::InconsistentSupersession {
            superseder: "EIP-2".to_string(),
            superseded: "EIP-1".to_string(),
        }));
    }

    #[test]
    fn detects_malformed_url() {
        let mut a = proposal("EIP-1");
        a.spec = "not-a-url".to_string();
        let errors = validate(&[loaded(a)]);
        assert!(
            errors.iter().any(
                |e| matches!(e, ValidationError::MalformedUrl { field, .. } if field == "spec")
            )
        );
    }

    #[test]
    fn detects_filename_mismatch() {
        let wrong = LoadedProposal {
            path: "data/ethereum/wrong.toml".into(),
            value: proposal("EIP-1"),
        };
        let errors = validate(&[wrong]);
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ValidationError::FilenameMismatch { .. }))
        );
    }
}
