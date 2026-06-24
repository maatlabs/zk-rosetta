#!/usr/bin/env bash
set -euo pipefail

# Reproduces verifier.runtime.hex: the deployed EVM bytecode of an audited
# Groth16 BN254 verifier parameterized by this vector's committed verifying key.
# Authors no cryptography. The verifier is gnark's MIT, audited Semaphore-lineage
# template; export_verifier.go feeds the committed snarkjs verifying key into
# gnark via vocdoni/circom2gnark, confirms the converted key/proof/public verify
# natively, and emits the Solidity. solc then compiles the deployed bytecode.
#
# Pinned toolchain: gnark v0.15.0, circom2gnark v1.0.0, solc 0.8.35.
# Run from this directory.

go mod init zkr-evm-verifier-export >/dev/null 2>&1 || true
go get github.com/vocdoni/circom2gnark@v1.0.0
go get github.com/consensys/gnark@v0.15.0
go mod tidy

go run export_verifier.go ../snarkjs Verifier.sol

solc --optimize --optimize-runs 200 --bin-runtime --evm-version cancun \
    Verifier.sol -o build --overwrite

cp build/Verifier.bin-runtime verifier.runtime.hex
echo >> verifier.runtime.hex
