//! Rendering the catalog dataset into a static site.
//!
//! The generator reads the catalog through [`zkr_catalog`], renders three kinds
//! of page with embedded [`minijinja`] templates---a filterable index, one
//! page per proposal (with prose rendered from Markdown), and the Rosetta
//! comparison view grouping proposals by shared primitive across ecosystems---
//! and writes them alongside the static assets under the output directory. All
//! intra-site links are relative, so the result is servable from any base path.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::Context;
use minijinja::{Environment, context};
use serde::Serialize;
use zkr_catalog::{Proposal, load_dir};

const BASE: &str = include_str!("../templates/base.html");
const INDEX: &str = include_str!("../templates/index.html");
const PROPOSAL: &str = include_str!("../templates/proposal.html");
const ROSETTA: &str = include_str!("../templates/rosetta.html");
const STYLE: &str = include_str!("../assets/style.css");
const FILTER_JS: &str = include_str!("../assets/filter.js");

/// A proposal enriched with the fields the templates need beyond the raw record.
#[derive(Serialize)]
struct ProposalView {
    #[serde(flatten)]
    proposal: Proposal,
    /// Lowercase id, used as the page filename and slug.
    slug: String,
    /// The `notes` prose rendered from Markdown to HTML.
    notes_html: String,
}

/// One Rosetta grouping: every catalogued surface of a single primitive.
#[derive(Serialize)]
struct RosettaGroup<'a> {
    primitive: String,
    /// Distinct ecosystems the primitive surfaces in, in display order.
    ecosystems: Vec<String>,
    proposals: Vec<&'a ProposalView>,
}

/// Renders the catalog under `data` into a static site at `out`.
///
/// Returns the number of proposals rendered. The output directory is replaced
/// wholesale so a build never leaves stale pages behind.
pub fn build(data: &Path, out: &Path) -> anyhow::Result<usize> {
    let views = load_dir(data)
        .with_context(|| format!("loading catalog from {}", data.display()))?
        .into_iter()
        .map(|entry| ProposalView {
            slug: entry.proposal.id.to_ascii_lowercase(),
            notes_html: markdown(&entry.proposal.notes),
            proposal: entry.proposal,
        })
        .collect::<Vec<_>>();

    let env = environment()?;

    if out.exists() {
        fs::remove_dir_all(out).with_context(|| format!("clearing {}", out.display()))?;
    }
    let proposals_dir = out.join("proposals");
    let assets_dir = out.join("assets");
    fs::create_dir_all(&proposals_dir)
        .with_context(|| format!("creating {}", proposals_dir.display()))?;
    fs::create_dir_all(&assets_dir)
        .with_context(|| format!("creating {}", assets_dir.display()))?;

    let index = env.get_template("index.html")?.render(context! {
        root => "",
        proposals => &views,
        ecosystems => distinct(&views, |v| Some(label(v.proposal.ecosystem))),
        layers => distinct(&views, |v| Some(label(v.proposal.layer))),
        categories => distinct(&views, |v| Some(label(v.proposal.category))),
        statuses => distinct(&views, |v| Some(label(v.proposal.status))),
        primitives => distinct(&views, |v| v.proposal.primitive.map(label)),
    })?;
    write(&out.join("index.html"), &index)?;

    let rosetta = env.get_template("rosetta.html")?.render(context! {
        root => "",
        groups => rosetta_groups(&views),
    })?;
    write(&out.join("rosetta.html"), &rosetta)?;

    let proposal_template = env.get_template("proposal.html")?;
    views.iter().try_for_each(|view| {
        let page = proposal_template.render(context! { root => "../", p => view })?;
        write(&proposals_dir.join(format!("{}.html", view.slug)), &page)
    })?;

    write(&assets_dir.join("style.css"), STYLE)?;
    write(&assets_dir.join("filter.js"), FILTER_JS)?;

    Ok(views.len())
}

fn environment() -> anyhow::Result<Environment<'static>> {
    let mut env = Environment::new();
    env.add_template("base.html", BASE)?;
    env.add_template("index.html", INDEX)?;
    env.add_template("proposal.html", PROPOSAL)?;
    env.add_template("rosetta.html", ROSETTA)?;
    Ok(env)
}

fn write(path: &Path, contents: &str) -> anyhow::Result<()> {
    fs::write(path, contents).with_context(|| format!("writing {}", path.display()))
}

fn markdown(src: &str) -> String {
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, pulldown_cmark::Parser::new(src));
    html
}

/// The serde wire form of an enum value, reused as its display label so the
/// site never duplicates the catalog's canonical naming.
fn label<T: Serialize>(value: T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_default()
}

/// The sorted, de-duplicated labels a projection yields across all proposals,
/// used to populate the index filter controls with only values in the data.
fn distinct(
    views: &[ProposalView],
    project: impl Fn(&ProposalView) -> Option<String>,
) -> Vec<String> {
    views
        .iter()
        .filter_map(project)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// Groups proposals by primitive, richest cross-ecosystem groups first.
fn rosetta_groups(views: &[ProposalView]) -> Vec<RosettaGroup<'_>> {
    let mut groups: BTreeMap<String, Vec<&ProposalView>> = BTreeMap::new();
    for view in views {
        if let Some(primitive) = view.proposal.primitive {
            groups.entry(label(primitive)).or_default().push(view);
        }
    }

    let mut groups = groups
        .into_iter()
        .map(|(primitive, proposals)| {
            let mut ecosystems = proposals
                .iter()
                .map(|view| label(view.proposal.ecosystem))
                .collect::<Vec<_>>();
            ecosystems.dedup();
            ecosystems.sort();
            ecosystems.dedup();
            RosettaGroup {
                primitive,
                ecosystems,
                proposals,
            }
        })
        .collect::<Vec<_>>();

    groups.sort_by(|a, b| {
        b.ecosystems
            .len()
            .cmp(&a.ecosystems.len())
            .then_with(|| a.primitive.cmp(&b.primitive))
    });
    groups
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use zkr_catalog::{Category, Ecosystem, Layer, Primitive, Relationships, Status};

    use super::*;

    const EIP: &str = r#"
id = "EIP-197"
title = "alt_bn128 pairing check"
ecosystem = "ethereum"
layer = "L1"
category = "primitive"
status = "final"
native_status = "Final"
primitive = "BN254"
enables = "Groth16 verification"
spec = "https://eips.ethereum.org/EIPS/eip-197"
notes = "Pairing check on **BN254**."

[[implementations]]
name = "alt-bn128 precompile"
language = "solidity"
url = "https://eips.ethereum.org/EIPS/eip-197"
audited = true
audit_ref = "https://example.com/audit"

[relationships]
equivalent_to = ["SIMD-0129"]
"#;

    const SIMD: &str = r#"
id = "SIMD-0129"
title = "alt_bn128 syscalls"
ecosystem = "solana"
layer = "L1"
category = "primitive"
status = "implemented"
native_status = "Implemented"
primitive = "BN254"
enables = "Groth16 verification"
spec = "https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0129-alt-bn128-syscalls.md"
notes = "Syscalls for BN254."

[relationships]
equivalent_to = ["EIP-197"]
"#;

    fn fixture() -> (PathBuf, PathBuf) {
        let unique = format!(
            "zkr-site-test-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
        );
        let base = std::env::temp_dir().join(unique);
        let data = base.join("data");
        fs::create_dir_all(data.join("ethereum")).unwrap();
        fs::create_dir_all(data.join("solana")).unwrap();
        fs::write(data.join("ethereum/eip-197.toml"), EIP).unwrap();
        fs::write(data.join("solana/simd-0129.toml"), SIMD).unwrap();
        (data, base.join("dist"))
    }

    fn sample(id: &str, ecosystem: Ecosystem, primitive: Option<Primitive>) -> Proposal {
        Proposal {
            id: id.into(),
            title: id.into(),
            ecosystem,
            layer: Layer::L1,
            category: Category::Primitive,
            status: Status::Final,
            native_status: "Final".into(),
            primitive,
            enables: "x".into(),
            spec: "https://example.com".into(),
            implementations: Vec::new(),
            relationships: Relationships::default(),
            sources: Vec::new(),
            notes: String::new(),
        }
    }

    #[test]
    fn renders_every_proposal_with_resolvable_links() {
        let (data, out) = fixture();
        let count = build(&data, &out).expect("build should succeed");
        assert_eq!(count, 2);

        let index = fs::read_to_string(out.join("index.html")).unwrap();
        assert!(index.contains("EIP-197") && index.contains("SIMD-0129"));
        // Filter controls are populated only from values present in the data.
        assert!(index.contains(">ethereum<") && index.contains(">solana<"));

        // Every proposal the index links to must actually exist on disk.
        for slug in ["eip-197", "simd-0129"] {
            assert!(
                out.join(format!("proposals/{slug}.html")).exists(),
                "missing page for {slug}"
            );
            assert!(index.contains(&format!("proposals/{slug}.html")));
        }

        assert!(out.join("assets/style.css").exists());
        assert!(out.join("assets/filter.js").exists());

        fs::remove_dir_all(out.parent().unwrap()).ok();
    }

    #[test]
    fn proposal_page_renders_markdown_and_relationships() {
        let (data, out) = fixture();
        build(&data, &out).unwrap();

        let page = fs::read_to_string(out.join("proposals/eip-197.html")).unwrap();
        // `notes` prose is rendered from Markdown, not emitted verbatim.
        assert!(page.contains("<strong>BN254</strong>"));
        // The equivalence edge links to the partner proposal's page.
        assert!(page.contains("proposals/simd-0129.html"));
        // The audited implementation surfaces its report link (hosts survive
        // HTML attribute escaping; the `/` separators are entity-encoded).
        assert!(page.contains("example.com") && page.contains(">report</a>"));

        fs::remove_dir_all(out.parent().unwrap()).ok();
    }

    #[test]
    fn rosetta_view_groups_a_shared_primitive_across_ecosystems() {
        let (data, out) = fixture();
        build(&data, &out).unwrap();

        let rosetta = fs::read_to_string(out.join("rosetta.html")).unwrap();
        assert!(rosetta.contains("BN254"));
        // The group spans both ecosystems and links both proposals.
        assert!(rosetta.contains("ethereum") && rosetta.contains("solana"));
        assert!(rosetta.contains("eip-197.html") && rosetta.contains("simd-0129.html"));

        fs::remove_dir_all(out.parent().unwrap()).ok();
    }

    #[test]
    fn groups_with_more_ecosystems_sort_first() {
        let shared = ProposalView {
            slug: "a".into(),
            notes_html: String::new(),
            proposal: sample("A", Ecosystem::Ethereum, Some(Primitive::Bn254)),
        };
        let partner = ProposalView {
            slug: "b".into(),
            notes_html: String::new(),
            proposal: sample("B", Ecosystem::Solana, Some(Primitive::Bn254)),
        };
        let lonely = ProposalView {
            slug: "c".into(),
            notes_html: String::new(),
            proposal: sample("C", Ecosystem::Ethereum, Some(Primitive::Kzg)),
        };
        let views = vec![shared, lonely, partner];

        let groups = rosetta_groups(&views);
        assert_eq!(groups[0].primitive, "BN254");
        assert_eq!(groups[0].ecosystems.len(), 2);
        assert_eq!(groups[1].primitive, "KZG");
    }
}
