<div align="center">
  <h1>ZK Rosetta</h1>
  <p><strong>A cross-ecosystem catalog of zero-knowledge protocol specifications---with executable proof that one statement verifies identically across chains.</strong></p>
</div>

<div align="center">

[![CI](https://github.com/maatlabs/zk-rosetta/actions/workflows/ci.yml/badge.svg)](https://github.com/maatlabs/zk-rosetta/actions/workflows/ci.yml)
[![Security Audit](https://github.com/maatlabs/zk-rosetta/actions/workflows/security.yml/badge.svg)](https://github.com/maatlabs/zk-rosetta/actions/workflows/security.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)
[![Releases](https://img.shields.io/github/v/release/maatlabs/zk-rosetta)](https://github.com/maatlabs/zk-rosetta/releases)
[![PRs welcome](https://img.shields.io/badge/PRs-welcome-ff69b4.svg?style=flat-square)](https://github.com/maatlabs/zk-rosetta/blob/main/CONTRIBUTING.md)

<strong><a href="https://maatlabs.github.io/zk-rosetta/">Browse the live catalog &rarr;</a></strong>

</div>

ZK Rosetta is a cross-ecosystem catalog of zero-knowledge protocols. Each entry maps to a spec or proposal, a normalized status, the cryptographic primitive it exposes, and its relationships to entries in other ecosystems, so that one primitive can be read across the conventions of different chains. The catalog spans the Ethereum, Bitcoin, and Solana base layers, the major ZK-native chains (Zcash, Filecoin, Starknet, Mina, Aleo, Penumbra, Namada, Midnight), the zkEVM rollups (Aztec, zkSync Era, Scroll, Polygon zkEVM, Linea, Taiko), and the cross-rollup RIP standards track, with cross-ecosystem equivalence clusters for shared primitives such as BN254 (the SNARK-verification curve shared by Ethereum, Solana, Aztec, and every catalogued zkEVM), BLS12-381 (Ethereum, Solana, Filecoin, Namada), and Poseidon (Zcash, Filecoin, Starknet, Mina, Aleo, Penumbra).

This project never authors cryptography. It catalogs protocol specs or proposals and links to audited implementations where they exist, recording where they do not; it does not implement, fork, or vendor any verifier, primitive, or curve operation.

## Repository layout

```text
zk-rosetta/
├── data/                  # catalog source: one TOML file per entry (data/<ecosystem>/<id>.toml)
├── crates/                # the Cargo workspace
│   ├── zkr-core           # shared infrastructure: primitive taxonomy, TOML loader, label helper
│   ├── zkr-catalog        # catalog data model, loader, validator (the Rust type is the schema)
│   ├── zkr-cli            # the zkr command-line tool (validate, schema, drift, vectors)
│   ├── zkr-site           # static-site generator: catalog index + Rosetta comparison view
│   └── zkr-harness        # parity harness: shared vector format + audited-verifier adapters
├── vectors/               # committed, ecosystem-neutral test vectors (each with provenance)
└── programs/              # on-chain SVM glue, built separately for the SBF runtime
```

## Usage

Validate the catalog:

```sh
cargo run -p zkr-cli -- validate
```

Pass `--online` to additionally check that every specification and implementation link resolves. Print the catalog-entry JSON Schema, derived from the Rust types, for editor and contributor tooling:

```sh
cargo run -p zkr-cli -- schema
```

Check every entry against its upstream proposal repository and report any that have drifted---a status or specification URL that no longer matches the source:

```sh
cargo run -p zkr-cli -- drift
```

A scheduled workflow runs the same check and opens (or updates) a single tracking issue when an entry falls out of sync with upstream; corrections are always made by hand through a pull request, so the dataset stays human-maintained.

Validate the committed parity test vectors for structural well-formedness, the same guard CI enforces:

```sh
cargo run -p zkr-cli -- vectors validate
```

Generate the static catalog site into `dist/`:

```sh
cargo run -p zkr-site -- build
```

The generated pages carry full-text search markup; the search index is produced at deploy time by [Pagefind](https://pagefind.app/), a build-time step that leaves the published site fully static with no backend.

## Parity harness

Beyond cataloging entries, the project demonstrates the Rosetta thesis executably: it drives audited verifiers over shared, ecosystem-neutral test vectors and shows that one statement verifies identically across ecosystems. Run the demonstration:

```sh
cargo test -p zkr-harness --all-features
```

Two curves are wired today. For BN254, the committed Groth16 vector is accepted by an audited verifier on the EVM (run in [`revm`](https://github.com/bluealloy/revm) through the `EIP-196`/`EIP-197` `alt_bn128` precompiles) and on the SVM (run in [`litesvm`](https://github.com/LiteSVM/litesvm) through the `sol_alt_bn128_*` syscalls), and a tampered proof is rejected by both. For BLS12-381, a committed BLS-signature vector is checked by Ethereum's `EIP-2537` pairing precompile (in `revm`) against Filecoin's audited [`bls-signatures`](https://github.com/filecoin-project/bls-signatures) library---the same relation on the same bytes across two ecosystems. zk-rosetta authors none of these verifiers; it drives audited implementations and asserts they agree.

Each catalogued equivalence links to the vector that proves it---a `proven_by` reference from the cluster to a `vectors/<name>` directory---and the [Rosetta comparison view](https://maatlabs.github.io/zk-rosetta/rosetta.html) marks a cluster as *proven parity* only when a committed vector drives audited verifiers to the same verdict on a fixed, parity-able curve; every other shared primitive is shown as a conceptual equivalence, so the site never implies a parity the math cannot back. See [vectors/README.md](./vectors/README.md) for the vector format and each vector's `PROVENANCE.md` for the audited verifiers behind every side.

## Contributing

Contributions are welcome. See [CONTRIBUTING.md](./CONTRIBUTING.md) for how to add or correct a catalog entry and open a pull request, and [CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md) for community expectations. To report a security issue, see [SECURITY.md](./SECURITY.md).

## License

Licensed under either of [Apache License, Version 2.0](./LICENSE-APACHE) or [MIT license](./LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
