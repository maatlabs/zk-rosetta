# bls12-381-bls-signature

A BLS12-381 signature over a fixed message, in the scheme Filecoin and the Ethereum consensus layer share: the public key lives in G1, the signature and the hashed message live in G2, and verification is the pairing relation `e(g1, signature) == e(public_key, message_hash)`. The message is committed already hashed to the curve, so every ecosystem verifies the identical relation on the identical bytes rather than re-running its own hash-to-curve. It is a positive (accepting) vector.

## How it was produced

Everything here was generated with external, audited tooling---this project authors no cryptography. The exact steps are in [`generate.sh`](./generate.sh), which builds and runs the committed [`generate.rs`](./generate.rs).

- **Signing stack:** [`bls-signatures`](https://github.com/filecoin-project/bls-signatures) `0.15.0` (Filecoin's library) on its `blst`/`blstrs` backend---`blstrs 0.7.1` over [`blst`](https://github.com/supranational/blst), the NCC-Group-audited BLS12-381 implementation Filecoin runs in production.
- **Deterministic inputs:** the secret key is derived from a fixed initialization keying material string and the message is a fixed byte string, both recorded in [`generate.rs`](./generate.rs), so re-running the generator reproduces this vector exactly. The message is hashed to G2 with the `BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_` ciphersuite, the scheme's standard hash-to-curve.
- **Self-check:** the generator calls `bls_signatures::verify` on the produced key, signature, and hash and refuses to emit a vector that does not verify.
- **Normalized form:** [`vector.toml`](./vector.toml) holds the public key, signature, and hashed message as uncompressed affine coordinates in this repository's ecosystem-neutral encoding (see [`../README.md`](../README.md)); G2 coordinates are written `[c0, c1]` (real part first), the mathematical order, and each adapter applies any reordering its ecosystem requires.

These keys are for test vectors only and must never be used in production.

## Audited verifiers for this vector

The relation is checked by audited, external code on each side---never by anything in this repository:

- **EVM:** the EIP-2537 `BLS12_PAIRING_CHECK` precompile, run in `revm` under the Prague spec. The harness poses the relation as the pairing product `e(-g1, signature) * e(public_key, message_hash) == 1`---the identical form the Filecoin verifier evaluates---and feeds the two pairs to the precompile, which returns acceptance. The negated G1 generator the product needs is the curve's standard generator with its y-coordinate negated, emitted by the same audited `blstrs` during generation.
- **Filecoin:** `bls_signatures::verify`, the production verifier itself, given the public key, signature, and the committed hash directly; it evaluates `e(g1, signature) == e(public_key, message_hash)`.

## Known gap

The Solana side of the BLS12-381 cluster (`SIMD-0388`, the `bls12_381` syscalls) is gated behind a runtime feature that is not yet activated on mainnet and whose proposal is still upstream `Review`. Until it activates there is no callable audited Solana verifier for this relation, so this vector demonstrates EVM-to-Filecoin parity only.
