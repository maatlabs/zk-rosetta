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

## Contributing

Contributions are welcome. See [CONTRIBUTING.md](./CONTRIBUTING.md) for how to add or correct a catalog entry and open a pull request, and [CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md) for community expectations. To report a security issue, see [SECURITY.md](./SECURITY.md).

## License

Licensed under either of [Apache License, Version 2.0](./LICENSE-APACHE) or [MIT license](./LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
