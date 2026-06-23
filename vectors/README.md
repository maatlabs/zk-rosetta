# Test vectors

Ecosystem-neutral test vectors for zero-knowledge primitives. Each vector pins one statement---a verifying key, a proof, and its public inputs---so that audited implementations on different ecosystems can be driven against the same bytes and shown to agree. The vectors are data only; the verification is always performed by external, audited code. This directory authors no cryptography.

## Layout

One directory per vector, named `<primitive>-<proof-system>-<statement>`:

- **`vector.toml`** --- the vector in the neutral encoding below; this is the file loaded by `zkr-harness`.
- **`PROVENANCE.md`** --- how the vector was produced, the exact tool versions, and the audited verifiers that check it.
- supporting files used to produce the vector (the circuit, the raw prover output, and the generation script), kept for transparency and independent re-verification.

## `vector.toml` format

```toml
proof_system = "groth16"   # the proof system the vk and proof belong to
primitive = "BN254"        # the curve, named as in the catalog's primitive taxonomy
expected = "accept"        # whether a correct verifier must accept or reject this vector

public_inputs = ["0x..."]  # the public signals, in statement order

[vk]                        # the verifying key
alpha_g1 = { x = "0x...", y = "0x..." }
beta_g2  = { x = ["0x...", "0x..."], y = ["0x...", "0x..."] }
gamma_g2 = { x = ["0x...", "0x..."], y = ["0x...", "0x..."] }
delta_g2 = { x = ["0x...", "0x..."], y = ["0x...", "0x..."] }
ic = [                      # one point per public input, plus one
  { x = "0x...", y = "0x..." },
  { x = "0x...", y = "0x..." },
]

[proof]
a = { x = "0x...", y = "0x..." }
b = { x = ["0x...", "0x..."], y = ["0x...", "0x..."] }
c = { x = "0x...", y = "0x..." }
```

## Encoding

- **Field elements** are `0x`-prefixed, fixed-width big-endian hex. For BN254 every element is 32 bytes (64 hex digits).
- **G1 points** are affine `{ x, y }`.
- **G2 points** carry coordinates over the quadratic extension field, written `{ x = [c0, c1], y = [c0, c1] }` where the element is `c0 + c1 * u`. This is the mathematical coordinate order; ecosystems that expect a different order (for example, the EVM `ecPairing` precompile's imaginary-first encoding) apply the reordering in their adapter, not here, so the vector stays neutral.
- **Verifying-key IC** holds exactly one more point than there are public inputs---the Groth16 invariant `ic.len() == public_inputs.len() + 1`.

## Adding a vector

Produce the vector with audited proving tooling (never by hand), commit the raw prover output alongside the normalized `vector.toml`, and record the toolchain and the audited verifiers in a `PROVENANCE.md`. `zkr-harness` loads and structurally checks every vector under this directory; the adapters then drive the audited verifiers against it.
