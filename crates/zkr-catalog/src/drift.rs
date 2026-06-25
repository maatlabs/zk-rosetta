//! Upstream freshness checking for catalog proposals.
//!
//! A catalog entry records a normalized [`Status`] (and a `spec` URL) that
//! can rot as the upstream proposal evolves. Drift checking compares each
//! entry against the live proposal repositories and reports the divergences.
//!
//! This module owns the pure half of that check: a per-ecosystem [`Source`]
//! locates a proposal's upstream document, reads its native status, and
//! normalizes it onto the catalog scale, and [`compare`] turns a fetched
//! document into [`Finding`]s. The network fetch lives in the caller, so the
//! whole surface here is exercised offline against committed fixtures. It
//! authors no cryptography.

use std::fmt;

use serde::Serialize;

use crate::model::{Ecosystem, Proposal, Status};

/// A per-ecosystem locator and parser for upstream proposal documents.
///
/// Implementations are pure: they derive URLs and read a fetched body but never
/// perform I/O. New ecosystems extend drift coverage by adding an implementation
/// and registering it in [`sources`].
pub trait Source {
    /// The ecosystem this source covers.
    fn ecosystem(&self) -> Ecosystem;

    /// The raw upstream document URL for `proposal`, or `None` when the source
    /// cannot locate it from the catalog entry.
    fn document_url(&self, proposal: &Proposal) -> Option<String>;

    /// The canonical spec URL the catalog should record, when the source can
    /// derive it independently of the recorded value; `None` skips spec drift.
    fn canonical_spec(&self, proposal: &Proposal) -> Option<String>;

    /// Reads and normalizes the upstream status from a fetched document body.
    fn parse(&self, body: &str) -> Result<Upstream, ParseError>;
}

/// An error reading the native status from a fetched upstream document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, thiserror::Error)]
#[serde(rename_all = "kebab-case")]
pub enum ParseError {
    /// The document carried no recognizable status field.
    #[error("no status field in upstream document")]
    NoStatusField,
    /// The status field held a value this source does not recognize.
    #[error("unrecognized upstream status `{0}`")]
    UnknownStatus(String),
}

/// The upstream status of a proposal, read from its source repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Upstream {
    /// The literal native status string.
    pub native: String,
    /// The native status normalized onto the catalog scale.
    pub status: Status,
}

/// Ethereum EIPs and ERCs, which split into separate repositories in 2023.
struct EthereumEips;

impl Source for EthereumEips {
    fn ecosystem(&self) -> Ecosystem {
        Ecosystem::Ethereum
    }

    fn document_url(&self, proposal: &Proposal) -> Option<String> {
        let slug = proposal.id.to_ascii_lowercase();
        if slug.starts_with("eip-") {
            Some(format!(
                "https://raw.githubusercontent.com/ethereum/EIPs/master/EIPS/{slug}.md"
            ))
        } else if slug.starts_with("erc-") {
            Some(format!(
                "https://raw.githubusercontent.com/ethereum/ERCs/master/ERCS/{slug}.md"
            ))
        } else {
            None
        }
    }

    fn canonical_spec(&self, proposal: &Proposal) -> Option<String> {
        let slug = proposal.id.to_ascii_lowercase();
        if slug.starts_with("eip-") {
            Some(format!("https://eips.ethereum.org/EIPS/{slug}"))
        } else if slug.starts_with("erc-") {
            Some(format!("https://ercs.ethereum.org/ERCS/{slug}"))
        } else {
            None
        }
    }

    fn parse(&self, body: &str) -> Result<Upstream, ParseError> {
        let native = front_matter_value(body, "status").ok_or(ParseError::NoStatusField)?;
        let status =
            normalize_eip(&native).ok_or_else(|| ParseError::UnknownStatus(native.clone()))?;
        Ok(Upstream { native, status })
    }
}

/// Bitcoin BIPs, located from the GitHub blob URL recorded as the spec.
struct BitcoinBips;

impl Source for BitcoinBips {
    fn ecosystem(&self) -> Ecosystem {
        Ecosystem::Bitcoin
    }

    fn document_url(&self, proposal: &Proposal) -> Option<String> {
        github_raw(&proposal.spec)
    }

    fn canonical_spec(&self, _proposal: &Proposal) -> Option<String> {
        None
    }

    fn parse(&self, body: &str) -> Result<Upstream, ParseError> {
        let native = preamble_value(body, "Status").ok_or(ParseError::NoStatusField)?;
        let status =
            normalize_bip(&native).ok_or_else(|| ParseError::UnknownStatus(native.clone()))?;
        Ok(Upstream { native, status })
    }
}

/// Solana SIMDs, located from the GitHub blob URL recorded as the spec.
struct SolanaSimds;

impl Source for SolanaSimds {
    fn ecosystem(&self) -> Ecosystem {
        Ecosystem::Solana
    }

    fn document_url(&self, proposal: &Proposal) -> Option<String> {
        github_raw(&proposal.spec)
    }

    fn canonical_spec(&self, _proposal: &Proposal) -> Option<String> {
        None
    }

    fn parse(&self, body: &str) -> Result<Upstream, ParseError> {
        let native = front_matter_value(body, "status").ok_or(ParseError::NoStatusField)?;
        let status =
            normalize_simd(&native).ok_or_else(|| ParseError::UnknownStatus(native.clone()))?;
        Ok(Upstream { native, status })
    }
}

/// Zcash ZIPs, an RST document per proposal in `zcash/zips`.
struct ZcashZips;

impl Source for ZcashZips {
    fn ecosystem(&self) -> Ecosystem {
        Ecosystem::Zcash
    }

    fn document_url(&self, proposal: &Proposal) -> Option<String> {
        let slug = proposal.id.to_ascii_lowercase();
        slug.starts_with("zip-")
            .then(|| format!("https://raw.githubusercontent.com/zcash/zips/main/zips/{slug}.rst"))
    }

    fn canonical_spec(&self, proposal: &Proposal) -> Option<String> {
        let slug = proposal.id.to_ascii_lowercase();
        slug.starts_with("zip-")
            .then(|| format!("https://zips.z.cash/{slug}"))
    }

    fn parse(&self, body: &str) -> Result<Upstream, ParseError> {
        // The ZIP header is an indented preamble inside an RST literal block,
        // so the BIP-style first-`Status:` reader applies.
        let native = preamble_value(body, "Status").ok_or(ParseError::NoStatusField)?;
        let status =
            normalize_zip(&native).ok_or_else(|| ParseError::UnknownStatus(native.clone()))?;
        Ok(Upstream { native, status })
    }
}

/// A single freshness divergence between the catalog and an upstream source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Finding {
    /// The recorded normalized status no longer matches upstream.
    StatusDrift {
        /// The proposal identifier.
        id: String,
        /// The status the catalog records.
        catalog: Status,
        /// The status the upstream document now carries.
        upstream: Status,
        /// The verbatim upstream status string behind `upstream`.
        native: String,
    },
    /// The recorded spec URL no longer matches the source's canonical form.
    SpecDrift {
        /// The proposal identifier.
        id: String,
        /// The spec URL the catalog records.
        catalog: String,
        /// The canonical spec URL the source derives.
        canonical: String,
    },
    /// The upstream document was fetched but its status could not be read.
    Unparsable {
        /// The proposal identifier.
        id: String,
        /// The document URL that was fetched.
        url: String,
        /// Why the status could not be read.
        error: ParseError,
    },
    /// The upstream document could not be fetched.
    Unreachable {
        /// The proposal identifier.
        id: String,
        /// The document URL that could not be fetched.
        url: String,
        /// The transport or status error reported by the fetcher.
        error: String,
    },
    /// The source covers the ecosystem but could not locate the document.
    NoLocator {
        /// The proposal identifier.
        id: String,
    },
    /// No drift source covers this proposal's ecosystem yet.
    NoSource {
        /// The proposal identifier.
        id: String,
        /// The uncovered ecosystem.
        ecosystem: Ecosystem,
    },
}

impl Finding {
    /// Whether the finding is genuine drift that should fail a check, as opposed
    /// to an informational coverage note (`NoLocator`, `NoSource`).
    pub fn is_drift(&self) -> bool {
        matches!(
            self,
            Self::StatusDrift { .. }
                | Self::SpecDrift { .. }
                | Self::Unparsable { .. }
                | Self::Unreachable { .. }
        )
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StatusDrift {
                id,
                catalog,
                upstream,
                native,
            } => write!(
                f,
                "drift {id}: status catalog={} upstream={} (native `{native}`)",
                zkr_core::label(*catalog),
                zkr_core::label(*upstream),
            ),
            Self::SpecDrift {
                id,
                catalog,
                canonical,
            } => write!(
                f,
                "drift {id}: spec catalog={catalog} canonical={canonical}"
            ),
            Self::Unparsable { id, url, error } => {
                write!(
                    f,
                    "drift {id}: unreadable upstream status at {url}: {error}"
                )
            }
            Self::Unreachable { id, url, error } => {
                write!(f, "drift {id}: upstream unreachable {url}: {error}")
            }
            Self::NoLocator { id } => write!(f, "note {id}: no upstream document locator"),
            Self::NoSource { id, ecosystem } => {
                write!(
                    f,
                    "note {id}: no drift source for {}",
                    zkr_core::label(*ecosystem)
                )
            }
        }
    }
}

/// Compares a proposal against the body fetched from its `document_url`,
/// returning every divergence (an empty vector when the entry is fresh).
pub fn compare(source: &dyn Source, proposal: &Proposal, url: &str, body: &str) -> Vec<Finding> {
    let status = match source.parse(body) {
        Ok(up) if up.status != proposal.status => Some(Finding::StatusDrift {
            id: proposal.id.clone(),
            catalog: proposal.status,
            upstream: up.status,
            native: up.native,
        }),
        Ok(_) => None,
        Err(error) => Some(Finding::Unparsable {
            id: proposal.id.clone(),
            url: url.to_string(),
            error,
        }),
    };
    let spec = source
        .canonical_spec(proposal)
        .filter(|canonical| *canonical != proposal.spec)
        .map(|canonical| Finding::SpecDrift {
            id: proposal.id.clone(),
            catalog: proposal.spec.clone(),
            canonical,
        });
    status.into_iter().chain(spec).collect()
}

/// The drift sources for every ecosystem the catalog currently covers.
pub fn sources() -> Vec<Box<dyn Source>> {
    vec![
        Box::new(EthereumEips),
        Box::new(BitcoinBips),
        Box::new(SolanaSimds),
        Box::new(ZcashZips),
    ]
}

/// The source covering `ecosystem`, if one is registered.
pub fn source_for(sources: &[Box<dyn Source>], ecosystem: Ecosystem) -> Option<&dyn Source> {
    sources
        .iter()
        .find(|source| source.ecosystem() == ecosystem)
        .map(|source| source.as_ref())
}

/// Reads a `key: value` field from a leading `---` YAML front-matter block.
fn front_matter_value(body: &str, key: &str) -> Option<String> {
    let mut lines = body.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }
    lines
        .take_while(|line| line.trim() != "---")
        .find_map(|line| field(line, key))
}

/// Reads an indented `Key: value` field from a BIP `<pre>` preamble.
fn preamble_value(body: &str, key: &str) -> Option<String> {
    body.lines().find_map(|line| field(line, key))
}

/// Parses `key: value` from one line, matching the key case-insensitively, and
/// returns the trimmed, unquoted value.
fn field(line: &str, key: &str) -> Option<String> {
    let (name, value) = line.split_once(':')?;
    name.trim().eq_ignore_ascii_case(key).then(|| {
        value
            .trim()
            .trim_matches(|c| c == '"' || c == '\'')
            .to_string()
    })
}

/// Converts a GitHub `blob` URL into its raw-content equivalent.
fn github_raw(spec: &str) -> Option<String> {
    let path = spec.strip_prefix("https://github.com/")?;
    Some(format!(
        "https://raw.githubusercontent.com/{}",
        path.replacen("/blob/", "/", 1)
    ))
}

/// Maps an EIP/ERC native status onto the normalized scale.
fn normalize_eip(native: &str) -> Option<Status> {
    Some(match native.to_ascii_lowercase().as_str() {
        "draft" => Status::Draft,
        "review" | "last call" => Status::Review,
        "accepted" => Status::Accepted,
        // `Living` standards (e.g. EIP-1) are continuously-maintained finals.
        "final" | "living" => Status::Final,
        "stagnant" => Status::Stagnant,
        "withdrawn" => Status::Withdrawn,
        _ => return None,
    })
}

/// Maps a BIP native status onto the normalized scale.
fn normalize_bip(native: &str) -> Option<Status> {
    Some(match native.to_ascii_lowercase().as_str() {
        "draft" => Status::Draft,
        "proposed" => Status::Accepted,
        // `Active` and `Deployed` (the value the Taproot BIPs carry) are live.
        "active" | "deployed" | "final" => Status::Final,
        "deferred" => Status::Stagnant,
        "rejected" | "withdrawn" => Status::Withdrawn,
        "replaced" | "obsolete" => Status::Superseded,
        _ => return None,
    })
}

/// Maps a SIMD native status onto the normalized scale.
fn normalize_simd(native: &str) -> Option<Status> {
    Some(match native.to_ascii_lowercase().as_str() {
        "idea" => Status::Idea,
        "draft" => Status::Draft,
        "review" => Status::Review,
        "accepted" => Status::Accepted,
        "implemented" => Status::Implemented,
        "activated" => Status::Final,
        "stagnant" => Status::Stagnant,
        "withdrawn" => Status::Withdrawn,
        _ => return None,
    })
}

/// Maps a ZIP native status onto the normalized scale.
fn normalize_zip(native: &str) -> Option<Status> {
    Some(match native.to_ascii_lowercase().as_str() {
        "reserved" => Status::Idea,
        "draft" => Status::Draft,
        "proposed" => Status::Accepted,
        "implemented" => Status::Implemented,
        // `Active` (process/informational ZIPs) and `Final` (a Consensus or
        // Standards ZIP activated on the network) are both live.
        "active" | "final" => Status::Final,
        "rejected" | "withdrawn" => Status::Withdrawn,
        "obsolete" => Status::Superseded,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load::parse_proposal;

    const EIP_197: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/drift/eip-197.md"
    ));
    const ERC_7812: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/drift/erc-7812.md"
    ));
    const BIP_340: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/drift/bip-0340.mediawiki"
    ));
    const SIMD_0129: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/drift/simd-0129.md"
    ));
    const ZIP_0224: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/drift/zip-0224.rst"
    ));

    fn proposal(id: &str, ecosystem: &str, status: &str, spec: &str) -> Proposal {
        let toml = format!(
            "id = \"{id}\"\ntitle = \"t\"\necosystem = \"{ecosystem}\"\nlayer = \"L1\"\n\
             category = \"primitive\"\nstatus = \"{status}\"\nnative_status = \"x\"\n\
             enables = \"e\"\nspec = \"{spec}\"\nnotes = \"n\"\n"
        );
        parse_proposal(&toml).expect("test proposal should parse")
    }

    #[test]
    fn parses_eip_status_from_real_front_matter() {
        let up = EthereumEips.parse(EIP_197).expect("eip fixture parses");
        assert_eq!(up.native, "Final");
        assert_eq!(up.status, Status::Final);
    }

    #[test]
    fn parses_erc_status_from_real_front_matter() {
        let up = EthereumEips.parse(ERC_7812).expect("erc fixture parses");
        assert_eq!(up.native, "Review");
        assert_eq!(up.status, Status::Review);
    }

    #[test]
    fn parses_bip_status_from_real_preamble() {
        // The Taproot BIPs carry the non-standard `Deployed` value, which the
        // mapping must fold to `final` so the seed shows no false drift.
        let up = BitcoinBips.parse(BIP_340).expect("bip fixture parses");
        assert_eq!(up.native, "Deployed");
        assert_eq!(up.status, Status::Final);
    }

    #[test]
    fn parses_simd_status_from_real_front_matter() {
        let up = SolanaSimds.parse(SIMD_0129).expect("simd fixture parses");
        assert_eq!(up.native, "Activated");
        assert_eq!(up.status, Status::Final);
    }

    #[test]
    fn parses_zip_status_from_real_rst_preamble() {
        let up = ZcashZips.parse(ZIP_0224).expect("zip fixture parses");
        assert_eq!(up.native, "Final");
        assert_eq!(up.status, Status::Final);
    }

    #[test]
    fn zcash_derives_raw_and_canonical_urls_from_the_padded_id() {
        let zip = proposal("ZIP-0224", "zcash", "final", "https://zips.z.cash/zip-0224");
        assert_eq!(
            ZcashZips.document_url(&zip).as_deref(),
            Some("https://raw.githubusercontent.com/zcash/zips/main/zips/zip-0224.rst")
        );
        assert_eq!(
            ZcashZips.canonical_spec(&zip).as_deref(),
            Some("https://zips.z.cash/zip-0224")
        );
    }

    #[test]
    fn normalization_tracks_each_ecosystem_vocabulary() {
        assert_eq!(normalize_eip("Last Call"), Some(Status::Review));
        assert_eq!(normalize_eip("Living"), Some(Status::Final));
        assert_eq!(normalize_eip("Moved"), None);
        assert_eq!(normalize_bip("Proposed"), Some(Status::Accepted));
        assert_eq!(normalize_bip("Replaced"), Some(Status::Superseded));
        assert_eq!(normalize_simd("Activated"), Some(Status::Final));
        assert_eq!(normalize_simd("Implemented"), Some(Status::Implemented));
        assert_eq!(normalize_zip("Active"), Some(Status::Final));
        assert_eq!(normalize_zip("Proposed"), Some(Status::Accepted));
        assert_eq!(normalize_zip("Obsolete"), Some(Status::Superseded));
    }

    #[test]
    fn unknown_status_value_is_reported_not_silently_dropped() {
        let body = "---\nstatus: Frozen\n---\n";
        assert_eq!(
            EthereumEips.parse(body),
            Err(ParseError::UnknownStatus("Frozen".into()))
        );
    }

    #[test]
    fn missing_status_field_is_reported() {
        assert_eq!(
            EthereumEips.parse("---\ntitle: x\n---\n"),
            Err(ParseError::NoStatusField)
        );
    }

    #[test]
    fn parsers_read_only_the_header_and_ignore_the_body() {
        // A `status` line below the closing front-matter fence belongs to the
        // body and must not be read as the header's status.
        assert_eq!(
            EthereumEips.parse("---\ntitle: x\n---\n\nstatus: Withdrawn\n"),
            Err(ParseError::NoStatusField)
        );
        // The first preamble `Status:` wins; a later one in the body is ignored.
        let bip = "<pre>\n  Status: Deployed\n</pre>\n\n  Status: Withdrawn\n";
        assert_eq!(
            BitcoinBips.parse(bip).map(|up| up.status),
            Ok(Status::Final)
        );
    }

    #[test]
    fn ethereum_routes_eip_and_erc_to_their_repositories() {
        let eip = proposal("EIP-197", "ethereum", "final", "https://x");
        let erc = proposal("ERC-7812", "ethereum", "review", "https://x");
        assert_eq!(
            EthereumEips.document_url(&eip).as_deref(),
            Some("https://raw.githubusercontent.com/ethereum/EIPs/master/EIPS/eip-197.md")
        );
        assert_eq!(
            EthereumEips.document_url(&erc).as_deref(),
            Some("https://raw.githubusercontent.com/ethereum/ERCs/master/ERCS/erc-7812.md")
        );
    }

    #[test]
    fn github_blob_urls_become_raw_urls() {
        assert_eq!(
            github_raw("https://github.com/bitcoin/bips/blob/master/bip-0340.mediawiki").as_deref(),
            Some("https://raw.githubusercontent.com/bitcoin/bips/master/bip-0340.mediawiki")
        );
        assert_eq!(
            github_raw("https://github.com/a/b/blob/main/proposals/0129-x.md").as_deref(),
            Some("https://raw.githubusercontent.com/a/b/main/proposals/0129-x.md")
        );
        assert_eq!(github_raw("https://example.com/x"), None);
    }

    #[test]
    fn compare_is_clean_when_status_and_spec_match() {
        let p = proposal(
            "EIP-197",
            "ethereum",
            "final",
            "https://eips.ethereum.org/EIPS/eip-197",
        );
        assert!(compare(&EthereumEips, &p, "url", EIP_197).is_empty());
    }

    #[test]
    fn compare_reports_status_drift() {
        let p = proposal(
            "EIP-197",
            "ethereum",
            "draft",
            "https://eips.ethereum.org/EIPS/eip-197",
        );
        let findings = compare(&EthereumEips, &p, "url", EIP_197);
        assert_eq!(
            findings,
            vec![Finding::StatusDrift {
                id: "EIP-197".into(),
                catalog: Status::Draft,
                upstream: Status::Final,
                native: "Final".into(),
            }]
        );
    }

    #[test]
    fn compare_reports_spec_drift_for_ethereum() {
        let p = proposal("EIP-197", "ethereum", "final", "https://example.com/wrong");
        let findings = compare(&EthereumEips, &p, "url", EIP_197);
        assert_eq!(
            findings,
            vec![Finding::SpecDrift {
                id: "EIP-197".into(),
                catalog: "https://example.com/wrong".into(),
                canonical: "https://eips.ethereum.org/EIPS/eip-197".into(),
            }]
        );
    }

    #[test]
    fn source_for_selects_by_ecosystem() {
        let sources = sources();
        assert_eq!(
            source_for(&sources, Ecosystem::Bitcoin).map(Source::ecosystem),
            Some(Ecosystem::Bitcoin)
        );
        assert_eq!(
            source_for(&sources, Ecosystem::Solana).map(Source::ecosystem),
            Some(Ecosystem::Solana)
        );
        assert_eq!(
            source_for(&sources, Ecosystem::Zcash).map(Source::ecosystem),
            Some(Ecosystem::Zcash)
        );
    }

    #[test]
    fn coverage_notes_are_not_treated_as_drift() {
        assert!(
            !Finding::NoSource {
                id: "X-1".into(),
                ecosystem: Ecosystem::Ethereum,
            }
            .is_drift()
        );
        assert!(
            Finding::StatusDrift {
                id: "EIP-1".into(),
                catalog: Status::Draft,
                upstream: Status::Final,
                native: "Final".into(),
            }
            .is_drift()
        );
    }
}
