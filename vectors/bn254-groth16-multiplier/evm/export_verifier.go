package main

import (
	"fmt"
	"os"
	"path/filepath"

	bn254fr "github.com/consensys/gnark-crypto/ecc/bn254/fr"
	groth16_bn254 "github.com/consensys/gnark/backend/groth16/bn254"
	"github.com/vocdoni/circom2gnark/parser"
)

// Emits a Solidity Groth16 verifier (gnark's MIT, audited Semaphore-lineage
// template) parameterized by a committed snarkjs verifying key, after first
// confirming the converted vk/proof/public verify natively under gnark.
func main() {
	if len(os.Args) != 3 {
		fmt.Fprintln(os.Stderr, "usage: evmgen <snarkjs-dir> <out-verifier.sol>")
		os.Exit(2)
	}
	dir, out := os.Args[1], os.Args[2]

	read := func(name string) []byte {
		b, err := os.ReadFile(filepath.Join(dir, name))
		if err != nil {
			panic(err)
		}
		return b
	}

	circomVk, err := parser.UnmarshalCircomVerificationKeyJSON(read("verification_key.json"))
	if err != nil {
		panic(err)
	}
	circomProof, err := parser.UnmarshalCircomProofJSON(read("proof.json"))
	if err != nil {
		panic(err)
	}
	publicSignals, err := parser.UnmarshalCircomPublicSignalsJSON(read("public.json"))
	if err != nil {
		panic(err)
	}

	vk, err := parser.ConvertVerificationKey(circomVk)
	if err != nil {
		panic(err)
	}
	proof, err := parser.ConvertProof(circomProof)
	if err != nil {
		panic(err)
	}
	pub, err := parser.ConvertPublicInputs(publicSignals)
	if err != nil {
		panic(err)
	}

	if err := groth16_bn254.Verify(proof, vk, bn254fr.Vector(pub)); err != nil {
		panic(fmt.Errorf("native gnark verify of converted artifacts failed: %w", err))
	}
	fmt.Fprintln(os.Stderr, "native gnark verify: OK")

	f, err := os.Create(out)
	if err != nil {
		panic(err)
	}
	defer f.Close()
	if err := vk.ExportSolidity(f); err != nil {
		panic(err)
	}
	fmt.Fprintf(os.Stderr, "wrote %s\n", out)
}
