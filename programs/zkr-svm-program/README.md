# zkr-svm-program

The SVM-side on-chain program for the zk-rosetta parity harness. It authors no cryptography: it deserializes a fixed instruction layout (a negated proof, a verifying key, and one public input) and forwards it to the audited [`groth16-solana`](https://github.com/Lightprotocol/groth16-solana) verifier, which performs the BN254 pairing check through the `sol_alt_bn128_*` syscalls. The harness runs the compiled program inside `litesvm` and reads its result: a verifying proof returns success, a non-verifying proof returns `Custom(1)`, and malformed instruction data returns `Custom(2)`.

This crate is intentionally outside the Cargo workspace (it is listed under `[workspace.exclude]`): it targets the SBF runtime with its own toolchain and dependency set, so it is built separately and never compiled for the host by the workspace's `cargo build`/`clippy`/`test`.

## Building

The committed `zkr_svm_program.so` is the artifact the harness loads. It is a reproducible build of this crate:

```bash
cd programs/zkr-svm-program
cargo build-sbf
cp target/deploy/zkr_svm_program.so zkr_svm_program.so
```

The build requires the Solana SBF toolchain (`cargo build-sbf`, from the Agave CLI). Rebuild and re-commit the `.so` whenever this crate's source or its `groth16-solana` dependency changes.

## Instruction layout

A single byte string, big-endian throughout, with no header:

| Bytes    | Field         | Notes                                   |
| -------- | ------------- | --------------------------------------- |
| 0..64    | `proof_a`     | G1, already negated by the harness      |
| 64..192  | `proof_b`     | G2, imaginary-part-first per coordinate |
| 192..256 | `proof_c`     | G1                                      |
| 256..288 | public input  | one 32-byte field element               |
| 288..352 | `vk_alpha_g1` | G1                                      |
| 352..480 | `vk_beta_g2`  | G2                                      |
| 480..608 | `vk_gamma_g2` | G2                                      |
| 608..736 | `vk_delta_g2` | G2                                      |
| 736..800 | `vk_ic[0]`    | G1                                      |
| 800..864 | `vk_ic[1]`    | G1                                      |

The G2 imaginary-part-first ordering and the `proof_a` negation are exactly what `groth16-solana` (and the underlying `sol_alt_bn128_pairing` syscall, which follows the EIP-197 ABI) expect; the harness performs both transformations from the ecosystem-neutral vector before invoking this program.
