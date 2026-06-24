//! On-chain SVM program for the zk-rosetta parity harness.
//!
//! This program authors no cryptography. It deserializes a fixed instruction
//! layout---a negated proof, a verifying key, and one public input---and
//! forwards it to the audited `groth16-solana` verifier, which runs the BN254
//! pairing check through the `sol_alt_bn128_*` syscalls. A verifying proof is
//! reported as `Ok(())`; a non-verifying proof as `Custom(VERIFY_FAILED)`; and
//! instruction data that does not match the layout as `Custom(BAD_INPUT)`.

// `solana-program`'s `entrypoint!` macro references custom-heap/custom-panic
// feature cfgs that this crate does not declare; the defaults it expands to are
// exactly what a program wants.
#![allow(unexpected_cfgs)]

use groth16_solana::groth16::{Groth16Verifier, Groth16Verifyingkey};
use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, program_error::ProgramError,
    pubkey::Pubkey,
};

/// Custom error code returned when the proof does not verify.
pub const VERIFY_FAILED: u32 = 1;
/// Custom error code returned when the instruction data is malformed.
pub const BAD_INPUT: u32 = 2;

// One negated G1 proof element, one G2, one G1, one public input, the four
// verifying-key points, and the two input-commitment basis points.
const EXPECTED_LEN: usize = 64 + 128 + 64 + 32 + 64 + 128 + 128 + 128 + 64 + 64;

entrypoint!(process_instruction);

fn process_instruction(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let input = Input::parse(instruction_data).ok_or(ProgramError::Custom(BAD_INPUT))?;
    let vk = Groth16Verifyingkey {
        nr_pubinputs: 1,
        vk_alpha_g1: input.vk_alpha,
        vk_beta_g2: input.vk_beta,
        vk_gamme_g2: input.vk_gamma,
        vk_delta_g2: input.vk_delta,
        vk_ic: &input.vk_ic,
    };
    Groth16Verifier::new(
        &input.proof_a,
        &input.proof_b,
        &input.proof_c,
        &input.public_inputs,
        &vk,
    )
    .and_then(|mut verifier| verifier.verify())
    .map_err(|_| ProgramError::Custom(VERIFY_FAILED))
}

// The instruction payload, fixed to a single-public-input BN254 Groth16 vector
// (hence the const-generic input count of one and the two basis points).
struct Input {
    proof_a: [u8; 64],
    proof_b: [u8; 128],
    proof_c: [u8; 64],
    public_inputs: [[u8; 32]; 1],
    vk_alpha: [u8; 64],
    vk_beta: [u8; 128],
    vk_gamma: [u8; 128],
    vk_delta: [u8; 128],
    vk_ic: [[u8; 64]; 2],
}

impl Input {
    fn parse(data: &[u8]) -> Option<Self> {
        if data.len() != EXPECTED_LEN {
            return None;
        }
        let (proof_a, rest) = data.split_at(64);
        let (proof_b, rest) = rest.split_at(128);
        let (proof_c, rest) = rest.split_at(64);
        let (public_input, rest) = rest.split_at(32);
        let (vk_alpha, rest) = rest.split_at(64);
        let (vk_beta, rest) = rest.split_at(128);
        let (vk_gamma, rest) = rest.split_at(128);
        let (vk_delta, rest) = rest.split_at(128);
        let (ic0, ic1) = rest.split_at(64);
        Some(Self {
            proof_a: proof_a.try_into().ok()?,
            proof_b: proof_b.try_into().ok()?,
            proof_c: proof_c.try_into().ok()?,
            public_inputs: [public_input.try_into().ok()?],
            vk_alpha: vk_alpha.try_into().ok()?,
            vk_beta: vk_beta.try_into().ok()?,
            vk_gamma: vk_gamma.try_into().ok()?,
            vk_delta: vk_delta.try_into().ok()?,
            vk_ic: [ic0.try_into().ok()?, ic1.try_into().ok()?],
        })
    }
}
