# Security Policy

## Scope and Threat Model

zk-rosetta is a catalog of zero-knowledge-related protocols and a harness that drives external, audited implementations to demonstrate cross-ecosystem parity. It never authors cryptography: it contains no verifiers, primitives, or curve arithmetic of its own. Cryptographic soundness therefore belongs to the upstream audited implementations the catalog links to and the harness invokes, not to this repository, and reports against those implementations should go to their respective maintainers.

The security surface this project owns is the glue around that data:

- **Catalog ingestion.** Catalog entry files are untrusted contributor input. They are parsed under strict deserialization (`deny_unknown_fields`) and checked against the catalog invariants before they are trusted. The parser and validator must handle malformed or adversarial TOML without panicking.
- **Link handling.** Specification, source, and implementation URLs are validated as well-formed `http`/`https` before use, and optional online checking performs outbound requests only when explicitly requested.
- **Harness orchestration.** The parity harness sits at a trust boundary between this project's orchestration code and the external implementations it executes. The harness owns only argument marshalling and result comparison; it must not weaken or reimplement any guarantee provided by the audited code it calls. It drives those implementations in-process---an audited EVM verifier under `revm` and an audited SVM verifier under `litesvm`---over committed test vectors.
- **Committed compiled artifacts.** Some audited implementations are executed from compiled artifacts committed to this repository (the EVM verifier bytecode and the on-chain SVM program). These are not opaque blobs: each is reproducible from committed source through the recipe and `PROVENANCE.md` recorded alongside it, so what the harness runs can be independently re-derived and audited rather than trusted on faith.

The glue code maintains the same discipline throughout: checked arithmetic, no `unwrap` in library paths, and no `unsafe`.

## Reporting a Vulnerability

If you discover a security issue in the catalog tooling or harness, please report it privately rather than opening a public issue. Use GitHub's [private vulnerability reporting](https://github.com/maatlabs/zk-rosetta/security/advisories/new) for this repository, or contact the maintainers directly.

Please include enough detail to reproduce the issue: affected version or commit, a minimal reproduction, and the impact you observe. We will acknowledge your report, investigate, and coordinate a fix and disclosure timeline with you.
