# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-06-24

The cross-ecosystem parity harness: zk-rosetta now demonstrates its thesis executably, driving audited verifiers over a shared test vector to prove that one statement verifies identically across ecosystems.

### Added

- Cross-ecosystem parity harness (`zkr-harness`): an ecosystem-neutral test-vector format (a verifying key, proof, and public inputs, with a documented encoding reusable for future primitives) and adapters that drive audited Groth16 BN254 verifiers on two ecosystems---the EVM, through the `EIP-196`/`EIP-197` `alt_bn128` precompiles, and the SVM, through the `sol_alt_bn128_*` syscalls.
- The first committed test vector: a BN254 Groth16 proof of the statement `3 * 11 = 33`, with full provenance recording the audited verifiers and exact toolchain behind each side.
- Parity demonstration: tests that the EVM and SVM verifiers reach the same verdict on the same bytes---both accepting the real proof and both rejecting a tampered one---so the equivalence is shown with executable test vectors rather than asserted in prose.

### Changed

- The catalog's primitive taxonomy now lives in a shared `zkr-core` crate alongside the common loading layer, so the catalog and the harness draw the same definitions. No change to the catalog data, the published schema, or any command's behavior.

## [0.1.0] - 2026-06-23

The first public release: a validated cross-ecosystem catalog of zero-knowledge-related protocol proposals, with command-line tooling and a generated catalog site.

### Added

- Catalog data model (`zkr-catalog`): a typed `Proposal` schema deserialized from per-proposal TOML files under `data/<ecosystem>/<id>.toml`, with the published JSON Schema derived directly from the Rust types.
- Catalog validator: strict deserialization plus invariant checks for unique identifiers, filename and directory consistency, well-formed specification, source, implementation, and audit URLs, referential integrity, symmetric equivalence edges, and consistent supersession.
- `zkr` command-line tool: `validate` (offline, with `--online` resolution of every catalogued link and retry of transient network failures) and `schema` (emits the proposal JSON Schema for editor and contributor tooling).
- Initial catalog covering Ethereum, Bitcoin, and Solana zero-knowledge-related proposals, each mapped to its canonical specification, a normalized status, the primitive it exposes, and its relationships.
- Cross-ecosystem equivalence edges and audited implementation links across the seed proposals, connecting proposals that expose the same primitive and recording where no audited implementation yet exists.
- Static-site generator (`zkr-site`): a filterable, sortable catalog index, a page per proposal with prose rendered from Markdown, and a Rosetta comparison view grouping proposals by shared primitive across ecosystems, with audited implementations shown side by side. Includes Pagefind full-text search and a GitHub Pages deployment workflow.
- Project documentation and governance: dual MIT and Apache-2.0 licensing, a contributing guide built around the catalog-entry workflow, a code of conduct, and a security policy.

---

## Guidelines for Contributors

When adding entries to this changelog for future releases:

1. **Format**: Follow [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
2. **Categories**: Use Added, Changed, Deprecated, Removed, Fixed, Security
3. **Audience**: Write for users, not developers (focus on impact, not implementation)
4. **Links**: Add comparison links at the bottom: `[0.2.0]: https://github.com/maatlabs/zk-rosetta/compare/v0.1.0...v0.2.0`

[0.2.0]: https://github.com/maatlabs/zk-rosetta/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/maatlabs/zk-rosetta/releases/tag/v0.1.0
