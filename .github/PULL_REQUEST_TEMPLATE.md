## Motivation
<!-- Please mention the issue fixed by this PR or detailed motivation -->
Closes #
<!-- `Closes #XXXX, closes #XXXX, ...` links mentioned issues to this PR and automatically closes them when it's merged -->

## Changes
<!-- Please describe in detail the changes made -->

## Test Plan
<!-- Please specify how these changes were tested (e.g. catalog validation, unit tests, manual testing) -->

## Checklist
<!-- This section should be removed when all items are complete -->
- [ ] Explain motivation or link existing issue(s)
- [ ] `cargo test --all-features --all-targets --workspace` passes
- [ ] `cargo run -p zkr-cli -- validate` reports no problems
- [ ] `cargo +nightly fmt` and `cargo clippy --all-features --all-targets --workspace -- -D warnings` are clean
- [ ] Documentation updated as needed
- [ ] This PR authors no cryptography; it only catalogs, links to, or drives audited implementations

## DevOps Notes
<!-- Please uncheck these items as applicable to make DevOps aware of changes that may affect releases -->
- [x] This PR does not require configuration changes (e.g., environment variables, GitHub secrets, VM resources)
- [x] This PR does not affect public APIs
- [x] This PR does not rely on a new version of external services
- [x] This PR does not make changes to log messages (which monitoring infrastructure may rely on)
