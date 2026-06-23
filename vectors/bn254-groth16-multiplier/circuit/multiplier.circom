pragma circom 2.0.0;

// The statement under test: knowledge of two factors a, b whose product is the
// public value c. This is the circuit only; the proving and verifying code is
// external, audited tooling (snarkjs, groth16-solana, the Semaphore verifier).
template Multiplier2() {
    signal input a;
    signal input b;
    signal output c;
    c <== a * b;
}

component main = Multiplier2();
