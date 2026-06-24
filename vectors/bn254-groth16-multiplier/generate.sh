#!/usr/bin/env bash
set -euo pipefail

# Reproduces the BN254/Groth16 multiplier test vector using external, audited
# tooling only: circom for the circuit, snarkjs for the trusted setup, proof, and
# the Semaphore-lineage Solidity verifier. The powers-of-tau here is a local
# test setup, not a secure multi-party ceremony, and the resulting keys are for
# test vectors only, never production. Run from a scratch directory.

POWER=8
ENTROPY="zk-rosetta test vector, not a production ceremony"

circom multiplier.circom --r1cs --wasm --sym

snarkjs powersoftau new bn128 "$POWER" pot_0000.ptau -v
snarkjs powersoftau contribute pot_0000.ptau pot_0001.ptau --name="zkr-test" -v -e="$ENTROPY"
snarkjs powersoftau prepare phase2 pot_0001.ptau pot_final.ptau -v

snarkjs groth16 setup multiplier.r1cs pot_final.ptau mult_0000.zkey
snarkjs zkey contribute mult_0000.zkey mult_final.zkey --name="zkr-test" -v -e="$ENTROPY"
snarkjs zkey export verificationkey mult_final.zkey verification_key.json

echo '{"a": "3", "b": "11"}' > input.json
node multiplier_js/generate_witness.js multiplier_js/multiplier.wasm input.json witness.wtns
snarkjs groth16 prove mult_final.zkey witness.wtns proof.json public.json
snarkjs groth16 verify verification_key.json public.json proof.json

snarkjs zkey export solidityverifier mult_final.zkey verifier.sol

node to_vector.mjs . > vector.toml
