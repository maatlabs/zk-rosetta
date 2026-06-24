# zk-rosetta

zk-rosetta is a cross-ecosystem catalog of zero-knowledge-related protocol proposals. Each entry maps a proposal to its specification, a normalized status, the cryptographic primitive it exposes, and its relationships to proposals in other ecosystems, so that one primitive can be read across the conventions of different chains.

zk-rosetta never authors cryptography. It catalogs proposals and links to audited implementations where they exist, recording where they do not; it does not implement, fork, or vendor any verifier, primitive, or curve operation.

## Repository layout

- `data/<ecosystem>/<id>.toml`: one file per proposal; the source the tooling reads.
- `crates/zkr-core`: shared infrastructure used across the other crates---the cryptographic-primitive taxonomy, the TOML loading layer, and the canonical display-label helper.
- `crates/zkr-catalog`: the proposal data model, loader, and validator. The Rust type is the schema.
- `crates/zkr-cli`: the `zkr` command-line tool.
- `crates/zkr-site`: the static-site generator that renders the catalog and the Rosetta comparison view.
- `crates/zkr-harness`: the cross-ecosystem parity harness---the shared test-vector format and the adapters that drive audited verifiers on each ecosystem.
- `vectors/<primitive>-<proof-system>-<statement>`: the committed, ecosystem-neutral test vectors the harness drives, each with its provenance.
- `programs/zkr-svm-program`: the on-chain (SVM) glue the harness drives---a small program that forwards a marshalled vector to an audited on-chain verifier. It targets the SBF runtime and is built separately with `cargo build-sbf` (kept out of the host workspace); the compiled program is committed so the harness can run it in `litesvm` without a Solana toolchain.

## Usage

Validate the catalog:

```sh
cargo run -p zkr-cli -- validate
```

Pass `--online` to additionally check that every specification and implementation link resolves. Print the proposal JSON Schema, derived from the Rust types, for editor and contributor tooling:

```sh
cargo run -p zkr-cli -- schema
```

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
