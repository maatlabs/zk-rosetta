//! SVM parity adapter.
//!
//! Drives the audited [`groth16-solana`](https://github.com/Lightprotocol/groth16-solana)
//! verifier inside `litesvm` by running the committed on-chain program
//! (`programs/zkr-svm-program`) over a vector. The adapter authors no
//! cryptography: it marshals the ecosystem-neutral vector into the program's
//! instruction layout---negating `proof_a` through arkworks and ordering G2
//! coordinates imaginary-part-first, both required by the `sol_alt_bn128_*`
//! syscall ABI---and reports whether the audited verifier accepted the proof.

use ark_bn254::{Fq, G1Affine};
use ark_ff::{BigInteger, PrimeField};
use litesvm::LiteSVM;
use solana_address::Address;
use solana_instruction::Instruction;
use solana_instruction_error::InstructionError;
use solana_keypair::Keypair;
use solana_signer::Signer;
use solana_transaction::Transaction;
use solana_transaction_error::TransactionError;

use crate::model::{Element, G1, G2, Primitive, ProofSystem, Vector};

/// The compiled on-chain program the adapter runs in `litesvm`, built from
/// `programs/zkr-svm-program` (see that crate's README for reproduction).
const PROGRAM: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../programs/zkr-svm-program/zkr_svm_program.so"
));

// Mirrors `zkr-svm-program`'s `VERIFY_FAILED` custom error code; a mismatch
// would surface as an `Unexpected` error in the reject test.
const VERIFY_FAILED: u32 = 1;

/// A fault in running the SVM adapter, as opposed to a verdict on the proof.
///
/// A rejected proof is reported as `Ok(false)` by [`verify`]; these variants
/// signal that the harness itself could not produce a verdict.
#[derive(Debug, thiserror::Error)]
pub enum SvmError {
    /// The vector is not a single-public-input BN254 Groth16 vector, the only
    /// shape this adapter's on-chain program is built for.
    #[error("vector is not a single-input BN254 Groth16 vector")]
    Unsupported,
    /// A field element was wider than the 32-byte BN254 field encoding.
    #[error("field element exceeds 32 bytes")]
    ElementTooWide,
    /// The verifier program could not be loaded into the VM.
    #[error("failed to load the verifier program: {0}")]
    Load(String),
    /// The VM could not be funded or otherwise set up.
    #[error("litesvm setup failed: {0}")]
    Setup(String),
    /// The program failed for a reason other than rejecting the proof.
    #[error("verifier program failed unexpectedly: {0}")]
    Unexpected(String),
}

/// Runs the audited SVM verifier over `vector`, returning whether it accepted.
///
/// `Ok(true)` means the on-chain `groth16-solana` verifier accepted the proof,
/// `Ok(false)` means it rejected it. An `Err` signals a harness or setup
/// fault, never a verdict on the proof itself.
pub fn verify(vector: &Vector) -> Result<bool, SvmError> {
    let data = marshal(vector)?;

    let mut svm = LiteSVM::new();
    let program_id = Address::new_unique();
    svm.add_program(program_id, PROGRAM)
        .map_err(|err| SvmError::Load(err.to_string()))?;

    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 1_000_000_000)
        .map_err(|failed| SvmError::Setup(failed.err.to_string()))?;

    let instruction = Instruction {
        program_id,
        accounts: Vec::new(),
        data,
    };
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        svm.latest_blockhash(),
    );

    match svm.send_transaction(transaction) {
        Ok(_) => Ok(true),
        Err(failed) => match failed.err {
            TransactionError::InstructionError(_, InstructionError::Custom(VERIFY_FAILED)) => {
                Ok(false)
            }
            other => Err(SvmError::Unexpected(other.to_string())),
        },
    }
}

/// Lays the vector out in the on-chain program's fixed instruction format.
fn marshal(vector: &Vector) -> Result<Vec<u8>, SvmError> {
    let [public_input] = vector.public_inputs.as_slice() else {
        return Err(SvmError::Unsupported);
    };
    let [ic0, ic1] = vector.vk.ic.as_slice() else {
        return Err(SvmError::Unsupported);
    };
    if vector.proof_system != ProofSystem::Groth16 || vector.primitive != Primitive::Bn254 {
        return Err(SvmError::Unsupported);
    }

    let sections: [&[u8]; 10] = [
        &negate_g1(&vector.proof.a)?,
        &g2_be(&vector.proof.b)?,
        &g1_be(&vector.proof.c)?,
        &fixed32(public_input)?,
        &g1_be(&vector.vk.alpha_g1)?,
        &g2_be(&vector.vk.beta_g2)?,
        &g2_be(&vector.vk.gamma_g2)?,
        &g2_be(&vector.vk.delta_g2)?,
        &g1_be(ic0)?,
        &g1_be(ic1)?,
    ];
    Ok(sections.concat())
}

/// A field element as exactly 32 big-endian bytes, left-padded as needed.
fn fixed32(element: &Element) -> Result<[u8; 32], SvmError> {
    pad_be(element.as_bytes())
}

/// A G1 point as `x || y`, 64 big-endian bytes.
fn g1_be(point: &G1) -> Result<[u8; 64], SvmError> {
    let mut out = [0u8; 64];
    out[..32].copy_from_slice(&fixed32(&point.x)?);
    out[32..].copy_from_slice(&fixed32(&point.y)?);
    Ok(out)
}

/// A G2 point as 128 big-endian bytes, each coordinate imaginary-part-first.
///
/// The vector stores coordinates as `[c0, c1]`; the `sol_alt_bn128_pairing`
/// syscall follows the EIP-197 ABI, which places the imaginary part first.
fn g2_be(point: &G2) -> Result<[u8; 128], SvmError> {
    let mut out = [0u8; 128];
    out[..32].copy_from_slice(&fixed32(&point.x[1])?);
    out[32..64].copy_from_slice(&fixed32(&point.x[0])?);
    out[64..96].copy_from_slice(&fixed32(&point.y[1])?);
    out[96..].copy_from_slice(&fixed32(&point.y[0])?);
    Ok(out)
}

/// A G1 point negated through arkworks, as `x || (-y)`, 64 big-endian bytes.
///
/// `groth16-solana` checks the pairing product against one, so it expects the
/// negation of `proof_a`. The negation is delegated to the audited `ark-bn254`
/// rather than computed here.
fn negate_g1(point: &G1) -> Result<[u8; 64], SvmError> {
    let x = Fq::from_be_bytes_mod_order(&fixed32(&point.x)?);
    let y = Fq::from_be_bytes_mod_order(&fixed32(&point.y)?);
    let negated = -G1Affine::new_unchecked(x, y);
    let mut out = [0u8; 64];
    out[..32].copy_from_slice(&be32(&negated.x)?);
    out[32..].copy_from_slice(&be32(&negated.y)?);
    Ok(out)
}

/// A field element as exactly 32 big-endian bytes, left-padded as needed.
fn be32(value: &Fq) -> Result<[u8; 32], SvmError> {
    pad_be(&value.into_bigint().to_bytes_be())
}

fn pad_be(bytes: &[u8]) -> Result<[u8; 32], SvmError> {
    let start = 32usize
        .checked_sub(bytes.len())
        .ok_or(SvmError::ElementTooWide)?;
    let mut out = [0u8; 32];
    out[start..].copy_from_slice(bytes);
    Ok(out)
}
