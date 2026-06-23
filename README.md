# zk-rosetta

zk-rosetta is a cross-ecosystem catalog of zero-knowledge-related protocol proposals. Each entry maps a proposal to its specification, a normalized status, the cryptographic primitive it exposes, and its relationships to proposals in other ecosystems, so that one primitive can be read across the conventions of different chains.

zk-rosetta never authors cryptography. It catalogs proposals and links to audited implementations where they exist, recording where they do not; it does not implement, fork, or vendor any verifier, primitive, or curve operation.

## Repository layout

- `data/<ecosystem>/<id>.toml`: one file per proposal; the source the tooling reads.
- `crates/zkr-catalog`: the proposal data model, loader, and validator. The Rust type is the schema.
- `crates/zkr-cli`: the `zkr` command-line tool.

## Usage

Validate the catalog:

```sh
cargo run -p zkr-cli -- validate
```

Pass `--online` to additionally check that every specification and implementation link resolves. Print the proposal JSON Schema, derived from the Rust types, for editor and contributor tooling:

```sh
cargo run -p zkr-cli -- schema
```

## License

Licensed under either of the Apache License, Version 2.0 or the MIT license, at your option.
