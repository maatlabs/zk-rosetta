# Security Policy

## Scope and Threat Model

zk-rosetta is a catalog of zero-knowledge-related protocol proposals and a harness that drives external, audited implementations to demonstrate cross-ecosystem parity. It never authors cryptography: it contains no verifiers, primitives, or curve arithmetic of its own. Cryptographic soundness therefore belongs to the upstream audited implementations the catalog links to and the harness invokes, not to this repository, and reports against those implementations should go to their respective maintainers.

The security surface this project owns is the glue around that data:

- **Catalog ingestion.** Proposal files are untrusted contributor input. They are parsed under strict deserialization (`deny_unknown_fields`) and checked against the catalog invariants before they are trusted. The parser and validator must handle malformed or adversarial TOML without panicking.
- **Link handling.** Specification, source, and implementation URLs are validated as well-formed `http`/`https` before use, and optional online checking performs outbound requests only when explicitly requested.
- **Harness orchestration.** As the parity harness grows, the boundary between this project's orchestration code and the external implementations it executes is a trust boundary. The harness owns only argument marshalling and result comparison; it must not weaken or reimplement any guarantee provided by the audited code it calls.

The glue code maintains the same discipline throughout: checked arithmetic, no `unwrap` in library paths, and no `unsafe`.

## Reporting a Vulnerability

If you discover a security issue in the catalog tooling or harness, please report it privately rather than opening a public issue. Use GitHub's [private vulnerability reporting](https://github.com/maatlabs/zk-rosetta/security/advisories/new) for this repository, or contact the maintainers directly.

Please include enough detail to reproduce the issue: affected version or commit, a minimal reproduction, and the impact you observe. We will acknowledge your report, investigate, and coordinate a fix and disclosure timeline with you.
