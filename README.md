<div align="center">
  <h1>ZK Rosetta</h1>
  <p><strong>A cross-ecosystem catalog of zero-knowledge protocol proposals---with executable proof that one statement verifies identically across chains.</strong></p>
</div>

<div align="center">

[![CI](https://github.com/maatlabs/zk-rosetta/actions/workflows/ci.yml/badge.svg)](https://github.com/maatlabs/zk-rosetta/actions/workflows/ci.yml)
[![Security Audit](https://github.com/maatlabs/zk-rosetta/actions/workflows/security.yml/badge.svg)](https://github.com/maatlabs/zk-rosetta/actions/workflows/security.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)
[![Releases](https://img.shields.io/github/v/release/maatlabs/zk-rosetta)](https://github.com/maatlabs/zk-rosetta/releases)
[![PRs welcome](https://img.shields.io/badge/PRs-welcome-ff69b4.svg?style=flat-square)](https://github.com/maatlabs/zk-rosetta/blob/main/CONTRIBUTING.md)

<strong><a href="https://maatlabs.github.io/zk-rosetta/">Browse the live catalog &rarr;</a></strong>

</div>

zk-rosetta is a cross-ecosystem catalog of zero-knowledge-related protocol proposals. Each entry maps a proposal to its specification, a normalized status, the cryptographic primitive it exposes, and its relationships to proposals in other ecosystems, so that one primitive can be read across the conventions of different chains. The catalog currently spans Ethereum, Bitcoin, Solana, Zcash, Filecoin, and Starknet, with cross-ecosystem equivalence clusters for shared primitives such as BLS12-381 (Ethereum, Solana, Filecoin) and Poseidon (Zcash, Filecoin, Starknet).

zk-rosetta never authors cryptography. It catalogs proposals and links to audited implementations where they exist, recording where they do not; it does not implement, fork, or vendor any verifier, primitive, or curve operation.

## Repository layout

```text
zk-rosetta/
├── data/                  # catalog source: one TOML file per proposal (data/<ecosystem>/<id>.toml)
├── crates/                # the Cargo workspace
│   ├── zkr-core           # shared infrastructure: primitive taxonomy, TOML loader, label helper
│   ├── zkr-catalog        # proposal data model, loader, validator (the Rust type is the schema)
│   ├── zkr-cli            # the zkr command-line tool (validate, schema, drift)
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

Pass `--online` to additionally check that every specification and implementation link resolves. Print the proposal JSON Schema, derived from the Rust types, for editor and contributor tooling:

```sh
cargo run -p zkr-cli -- schema
```

Check every entry against its upstream proposal repository and report any that have drifted---a status or specification URL that no longer matches the source:

```sh
cargo run -p zkr-cli -- drift
```

A scheduled workflow runs the same check and opens (or updates) a single tracking issue when an entry falls out of sync with upstream; corrections are always made by hand through a pull request, so the dataset stays human-maintained.

Generate the static catalog site into `dist/`:

```sh
cargo run -p zkr-site -- build
```

The generated pages carry full-text search markup; the search index is produced at deploy time by [Pagefind](https://pagefind.app/), a build-time step that leaves the published site fully static with no backend.

## Parity harness

Beyond cataloging proposals, zk-rosetta demonstrates the Rosetta thesis executably: it drives audited verifiers over a shared, ecosystem-neutral test vector and shows that one statement verifies identically across ecosystems. Run the demonstration:

```sh
cargo test -p zkr-harness --all-features
```

This loads the committed BN254 Groth16 vector and checks that an audited verifier on the EVM (run in [`revm`](https://github.com/bluealloy/revm) through the `EIP-196`/`EIP-197` `alt_bn128` precompiles) and an audited verifier on the SVM (run in [`litesvm`](https://github.com/LiteSVM/litesvm) through the `sol_alt_bn128_*` syscalls) both accept the proof and both reject a tampered one---the same verdict on the same bytes. zk-rosetta authors none of these verifiers; it drives audited implementations and asserts they agree. See [vectors/README.md](./vectors/README.md) for the vector format and [the vector's provenance](./vectors/bn254-groth16-multiplier/PROVENANCE.md) for the audited verifiers behind each side.

## Contributing

Contributions are welcome. See [CONTRIBUTING.md](./CONTRIBUTING.md) for how to add or correct a catalog entry and open a pull request, and [CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md) for community expectations. To report a security issue, see [SECURITY.md](./SECURITY.md).

## License

Licensed under either of [Apache License, Version 2.0](./LICENSE-APACHE) or [MIT license](./LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
