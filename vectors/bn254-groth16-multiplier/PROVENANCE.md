# bn254-groth16-multiplier

A BN254 Groth16 proof of the statement *"I know two factors `a`, `b` whose product is the public value `c`"*, instantiated with `a = 3`, `b = 11`, `c = 33`. The vector exists to be verified identically by audited implementations on more than one ecosystem; it is a positive (accepting) vector.

## How it was produced

Everything here was generated with external, audited tooling---this project authors no cryptography. The exact steps are in [`generate.sh`](./generate.sh) and the snarkjs-to-vector marshalling is in [`to_vector.mjs`](./to_vector.mjs).

- **Circuit:** [`circuit/multiplier.circom`](./circuit/multiplier.circom), the statement only (an arithmetic constraint, `c === a * b`), compiled with `circom 2.2.3`.
- **Proving stack:** `snarkjs 0.7.6` over the BN254 (`bn128`) curve---a local powers-of-tau, the Groth16 setup, and the proof.
- **Raw tool output (re-verifiable):** [`snarkjs/verification_key.json`](./snarkjs/verification_key.json), [`snarkjs/proof.json`](./snarkjs/proof.json), and [`snarkjs/public.json`](./snarkjs/public.json) are committed verbatim so anyone can independently re-check the vector with `snarkjs groth16 verify verification_key.json public.json proof.json`.
- **Normalized form:** [`vector.toml`](./vector.toml) is the same verifying key, proof, and public inputs in this repository's ecosystem-neutral encoding (see [`../README.md`](../README.md)).

The powers-of-tau here is a local, single-contribution test setup, not a secure multi-party ceremony. These keys are for test vectors only and must never be used in production.

Because a Groth16 trusted setup draws fresh randomness, re-running `generate.sh` produces a different but equally valid verifying key and proof; the committed files are the canonical artifacts for this vector.

## Audited verifiers for this verifying key

The statement is checked by audited, external code on each side---never by anything in this repository:

- **EVM:** the Groth16 verifier in [`evm/`](./evm/) is the audited Semaphore-lineage BN254 verifier template (the malleability-fixed contract carrying a third-party security audit, the same lineage Least Authority audited for World), parameterized by the verifying key committed here. It is obtained under its MIT license via gnark's `ExportSolidity` (the Apache-2.0, audited Consensys library), avoiding the GPL-3.0 output of `snarkjs zkey export solidityverifier`. Because the proving key is not committed --- and re-running the trusted setup would draw fresh randomness that no longer matches this vector --- the verifier is derived from the committed *verifying key*, not the proving key: [`evm/export_verifier.go`](./evm/export_verifier.go) feeds `verification_key.json` through [`vocdoni/circom2gnark`](https://github.com/vocdoni/circom2gnark) into gnark, confirms the converted key, proof, and public input verify natively, and emits the Solidity, which `solc` compiles to the deployed bytecode committed as [`evm/verifier.runtime.hex`](./evm/verifier.runtime.hex). The audited source is regenerated rather than vendored (see [`evm/generate.sh`](./evm/generate.sh)); the harness runs the committed bytecode in `revm` through the real EIP-196/197 `alt_bn128` precompiles.
- **SVM:** `groth16-solana` (Light Protocol), an audited consumer of the `sol_alt_bn128_*` syscalls, verifies snarkjs-shaped BN254 Groth16 proofs directly.
