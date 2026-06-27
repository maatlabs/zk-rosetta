# Contributing Guidelines

Thank you for your interest in contributing to this project. Whether it's a bug report, a new catalog entry, a correction, or additional documentation, feedback and contributions from the community are greatly valued.

Please read through this document before submitting any issues or pull requests to ensure reviewers have all the necessary information to effectively respond to your bug report or contribution.

## Reporting Bugs/Feature Requests

Kindly use the GitHub issue tracker to report bugs or suggest features.

When filing an issue, please check [existing open](https://github.com/maatlabs/zk-rosetta/issues), or [recently closed](https://github.com/maatlabs/zk-rosetta/issues?utf8=%E2%9C%93&q=is%3Aissue%20is%3Aclosed%20) issues to make sure somebody else hasn't already reported the issue. Please try to include as much information as you can. Details like these are incredibly useful:

* A reproducible test case or series of steps
* The version of the code being used
* Any modifications you've made relevant to the bug
* Anything unusual about your environment or deployment

## Adding or Correcting a Catalog Entry

The catalog is the heart of this project, so entries are the most common contribution. Each entry is a single TOML file under `data/<ecosystem>/<id>.toml`, deserialized into the `Entry` type in `crates/zkr-catalog`; that Rust type is the schema, and the validator is the gate every entry must pass. An entry maps either a numbered improvement proposal (`kind = "proposal"`, the default) or a section of a chain's protocol specification (`kind = "spec"`) to its canonical specification and the audited implementations that realize it. To add or change one:

1. Add or edit the relevant `data/<ecosystem>/<id>.toml` file. The filename stem must match the entry `id` (lowercased), and the file must live under the directory for its `ecosystem`. A `kind = "proposal"` entry uses its native id (`EIP-197`, `MIP-0003`); a `kind = "spec"` entry uses an `<ecosystem>-<feature>` slug and points `spec` at the canonical protocol specification. Record the normalized `status` on the shared scale alongside the verbatim `native_status`, the `category` and `primitive`, the canonical `spec` URL, and the `sources` you read; cross-ecosystem `equivalent_to` edges and supersession links must be added on both endpoints.
2. Run `cargo run -p zkr-cli -- validate` and resolve every reported problem. The validator enforces unique identifiers, filename and directory consistency, well-formed `http`/`https` URLs, referential integrity, and symmetric cross-references. This is the same check CI runs on your pull request, so a clean local run is the fastest path to a green build.
3. Optionally run `cargo run -p zkr-cli -- validate --online` to confirm that every `spec`, `sources`, and implementation URL still resolves before you submit.
4. Cite a canonical source for every status, title, and relationship you record. This project never authors cryptography: it links to specifications and audited implementations rather than reproducing them, and an entry with no audited implementation records that absence honestly rather than filling it.

The catalog-entry JSON Schema, useful for editor tooling, is available via `cargo run -p zkr-cli -- schema`.

## Adding or Correcting an Audited-Implementation Link

A catalog entry is only as valuable as the audited code it points to. Each implementation is one item in the entry's `implementations` array, with `name`, `language`, `url`, an `audited` boolean, and an `audit_ref` linking the audit report or commit when one exists. To add or correct one:

1. Set `audited = true` only when you can cite a public audit in `audit_ref`; otherwise record `audited = false` so the gap stays visible. Never paper over a missing or unaudited implementation with code of our own.
2. Prefer linking the upstream audited library, on-chain precompile, or runtime syscall directly over any fork or re-host: this project depends on audited code, it does not vendor it.
3. Run `cargo run -p zkr-cli -- validate` (and `validate --online` to confirm the new `url` and `audit_ref` resolve) before submitting.

## Keeping the Catalog Fresh

Upstream proposals change status and move over time, so a recorded `status` or `spec` can rot. `cargo run -p zkr-cli -- drift` compares every proposal entry against its upstream proposal repository and reports any divergence (spec entries document a protocol section rather than a numbered proposal, so they are not drift-tracked); a scheduled job runs the same check and opens a single tracking issue when it finds drift. Corrections always flow through a normal pull request that edits `data/`---the dataset stays human-maintained, and the automation only ever reports, never writes to the catalog.

## Contributing via Pull Requests

Contributions via pull requests are much appreciated. Before sending a pull request, please ensure that:

1. You are working against the latest source on the *main* branch.
2. You check existing open, and recently merged, pull requests to make sure someone else hasn't addressed the problem already.
3. Furthermore, you open an issue to discuss any significant work.

To send a pull request, please:

1. Fork the repository.
2. Modify the source; please focus on the specific change you are contributing. If you also reformat all the code, it will be hard for reviewers to focus on your change.
3. Ensure local tests pass and the catalog validates.
4. Commit to your fork using clear commit messages.
5. Send a pull request, answering any default questions in the pull request interface.
6. Pay attention to any automated CI failures reported in the pull request, and stay involved in the conversation.

GitHub provides additional document on [forking a repository](https://help.github.com/articles/fork-a-repo/) and
[creating a pull request](https://help.github.com/articles/creating-a-pull-request/).

## Licensing

See the [LICENSE-MIT](./LICENSE-MIT) and [LICENSE-APACHE](./LICENSE-APACHE) files for this project's licensing. Kindly confirm the licensing of your contribution. You may be asked to sign a [Contributor License Agreement (CLA)](http://en.wikipedia.org/wiki/Contributor_License_Agreement) for larger changes.
