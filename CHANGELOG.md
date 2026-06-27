# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - 2026-06-27

Broadens coverage from six ecosystems to eighteen. The Rosetta view now distinguishes harness-proven parity from conceptual equivalence, a second curve (BLS12-381) joins the parity harness alongside BN254, the catalog links each demonstrated equivalence to the committed vector that proves it, and coverage expands across the major ZK-native layer-one chains and zkEVM rollups.

### Added

- Catalog entries can now link a cross-ecosystem equivalence to the committed test vector that proves it, via a `proven_by` field naming a directory under `vectors/`. `zkr validate` checks every such reference, rejecting one that names a missing vector or a vector whose primitive does not match the entry, so the catalog cannot claim a parity no committed vector backs.
- The Rosetta comparison view now marks each primitive as proven parity or conceptual equivalence: a primitive earns the proven-parity badge only when it is a fixed, parity-able curve and a committed vector drives audited verifiers on each ecosystem to the same verdict. A short legend explains the distinction, so the site never implies a parity that the underlying math cannot back.
- `zkr vectors validate` checks every committed parity test vector for structural well-formedness, reporting any problem against its file path in a stable order and exiting non-zero on failure, so contributors can run the same guard locally that CI enforces.
- The parity harness now spans a second curve, BLS12-381. A committed BLS signature vector is verified identically by Ethereum's EIP-2537 pairing precompile (in `revm`) and Filecoin's audited `bls-signatures`, demonstrating the same relation on the same bytes across two ecosystems; the Rosetta view marks the BLS12-381 cluster as proven parity. The Solana side (the `SIMD-0388` BLS syscalls) is recorded as a gap until those syscalls activate. The test-vector format now carries either a Groth16 SNARK statement or a BLS signature statement, generalizing it beyond its original Groth16-only shape.
- `zkr drift` now recognizes when an Ethereum proposal has moved between the EIP and ERC repositories, reporting the relocation and its new location instead of a dead link.
- Catalog coverage for Mina, the first ZK-native L1 of this release's coverage wave: its Kimchi proof system (MIP-0003), mapped to its canonical specification and joined to the cross-ecosystem Poseidon cluster alongside Zcash, Filecoin, and Starknet. `zkr drift` now tracks Mina MIPs against their upstream repository.
- Catalog coverage for five more ZK-native chains, each catalogued as a protocol-spec entry mapped to its canonical specification: Aleo (the Varuna proof system) and Penumbra (Groth16 shielded transactions over decaf377) join the cross-ecosystem Poseidon cluster; Aztec (the UltraHonk proof system) joins the BN254 cluster; Namada (the Sapling-derived Multi-Asset Shielded Pool) joins the BLS12-381 cluster; and Midnight (a Halo2 system built on KZG commitments) pairs with Ethereum's EIP-4844 to form the catalog's first cross-ecosystem KZG link. Together with Mina, this completes the release's coverage of ZK-native layer-one chains.
- Catalog coverage for the major zkEVM rollups and the cross-rollup standards track: zkSync Era (Boojum), Scroll (Halo2), Polygon zkEVM (Plonky2), Linea (gnark PLONK), and Taiko (zkVM-based) each join the BN254 cluster, because however different their provers, all compress their validity proofs into a BN254 SNARK that Ethereum's EIP-197 pairing precompile verifies on L1. The first Rollup Improvement Proposal, RIP-7212 (the secp256r1 precompile), is catalogued with its supersession by EIP-7951 recorded, and `zkr drift` now tracks RIPs against their upstream repository.
- Catalog entries now declare a `kind`: a numbered improvement `proposal` (the default, freshness-tracked by `zkr drift`) or a protocol-`spec` section (identified by a stable slug, its reachability covered by `validate --online` rather than drift). This lets ZK-native chains that specify their cryptography in a protocol specification, rather than a numbered improvement proposal, be catalogued faithfully. Existing entries deserialize unchanged; the derived JSON Schema gains the `kind` field and its root type is now `Entry` (renamed from `Proposal`), and a spec entry is labelled as such on its catalog page.

### Changed

- `zkr drift` now also verifies the specification URL of Filecoin (FIP) and Starknet (SNIP) entries against their canonical form, broadening specification-drift coverage to four ecosystems alongside Ethereum and Zcash.
- `zkr validate --online` resolves catalogued links in parallel, cutting a full online check from over a minute to a few seconds, with identical, order-stable output.
- The published site builds its search index from a pinned, checksum-verified Pagefind binary, so the deployment is reproducible rather than tracking the latest released tool.
- Generated site links render as clean URLs, with path separators no longer HTML-entity-encoded; rendered behavior is unchanged.

## [0.3.0] - 2026-06-25

Freshness automation and coverage for ZK-native chains: zk-rosetta now checks itself against the upstream proposal repositories and extends past the Ethereum/Bitcoin/Solana seed into three ZK-native ecosystems (Zcash, Filecoin, StarkNet).

### Added

- `zkr drift`: a command that checks every catalog entry against its upstream proposal repository---EIP/ERC, BIP, SIMD, ZIP, FIP, and SNIP---and reports any divergence in normalized status or specification URL, in a human-readable or JSON format, exiting non-zero on genuine drift.
- A scheduled freshness workflow: a cron-triggered job runs `zkr drift` and opens or updates a single tracking issue when an entry falls out of sync with upstream, closing it once the catalog matches again. The automation only ever reports; corrections are made by hand through a pull request, so the dataset stays human-maintained.
- Catalog coverage for three ZK-native ecosystems: Zcash (the Orchard shielded protocol and the canonical Jubjub encoding rule), Filecoin (on-chain BLS aggregate signatures and Non-Interactive PoRep), and Starknet (typed-data signing in the style of EIP-712 and the standard account interface), each linked to audited implementations where they exist and recording the gaps where they do not.
- Cross-ecosystem equivalence clusters for newly shared primitives: BLS12-381 now spans Ethereum, Solana, and Filecoin, and Poseidon becomes the first primitive linked across Zcash, Filecoin, and Starknet.
- The `Jubjub` primitive in the cross-ecosystem taxonomy.
- A contributor guide for the catalog workflow: end-to-end steps for adding a proposal or an audited-implementation link, with the `validate` gate enforced in CI.

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
4. **Links**: Add comparison links at the bottom, e.g.: `[0.4.0]: https://github.com/maatlabs/zk-rosetta/compare/v0.3.0...v0.4.0`

[0.4.0]: https://github.com/maatlabs/zk-rosetta/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/maatlabs/zk-rosetta/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/maatlabs/zk-rosetta/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/maatlabs/zk-rosetta/releases/tag/v0.1.0
