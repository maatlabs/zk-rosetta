//! EVM parity adapter.
//!
//! Drives an audited Groth16 BN254 verifier inside the `revm` in-process EVM.
//! The verifier is gnark's MIT, audited Semaphore-lineage Solidity template
//! parameterized by the committed vector's verifying key, compiled once to the
//! deployed bytecode committed alongside the vector (see that directory's
//! `generate.sh`). The adapter authors no cryptography: it marshals the
//! ecosystem-neutral vector into the verifier's `verifyProof(bytes,uint256[1])`
//! calldata---G2 coordinates imaginary-part-first per the EIP-197 ABI, the G1
//! `proof_a` passed unnegated because the contract negates it internally---runs
//! it through the real EIP-196/197 `alt_bn128` precompiles, and reports whether
//! the verifier accepted (the contract reverts `ProofInvalid()` on rejection).
//!
//! The committed bytecode bakes in this vector's verifying key, so the adapter
//! verifies the proof and public input of that vector; a different verifying key
//! requires regenerating the bytecode.

use revm::context::result::ExecutionResult;
use revm::context::{Context, TxEnv};
use revm::database::{CacheDB, EmptyDB};
use revm::primitives::{Bytes, TxKind, U256, address, keccak256};
use revm::state::{AccountInfo, Bytecode};
use revm::{ExecuteEvm, MainBuilder, MainContext};

use crate::model::{Element, G1, G2, Primitive, ProofSystem, Vector};

/// The audited verifier's deployed bytecode, compiled from the committed vector's
/// verifying key (see the vector's `evm/generate.sh` for reproduction).
const VERIFIER_RUNTIME_HEX: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../vectors/bn254-groth16-multiplier/evm/verifier.runtime.hex"
));

/// A fault in running the EVM adapter, as opposed to a verdict on the proof.
///
/// A rejected proof is reported as `Ok(false)` by [`verify`]; these variants
/// signal that the harness itself could not produce a verdict.
#[derive(Debug, thiserror::Error)]
pub enum EvmError {
    /// The vector is not a single-public-input BN254 Groth16 vector, the only
    /// shape this adapter's committed verifier is built for.
    #[error("vector is not a single-input BN254 Groth16 vector")]
    Unsupported,
    /// A field element was wider than the 32-byte BN254 field encoding.
    #[error("field element exceeds 32 bytes")]
    ElementTooWide,
    /// The committed verifier bytecode could not be decoded.
    #[error("failed to decode the verifier bytecode: {0}")]
    Bytecode(String),
    /// The transaction could not be built or executed.
    #[error("revm execution failed: {0}")]
    Execution(String),
    /// The verifier reverted or halted for a reason other than rejecting the
    /// proof, or returned output the adapter did not expect.
    #[error("verifier failed unexpectedly: {0}")]
    Unexpected(String),
}

/// Runs the audited EVM verifier over `vector`, returning whether it accepted.
///
/// `Ok(true)` means the on-chain Groth16 verifier accepted the proof, `Ok(false)`
/// means it rejected it (the contract reverted `ProofInvalid()`). An `Err`
/// signals a harness or setup fault, not a verdict on the proof itself.
pub fn verify(vector: &Vector) -> Result<bool, EvmError> {
    let data = calldata(vector)?;

    let runtime = hex::decode(VERIFIER_RUNTIME_HEX.trim())
        .map_err(|err| EvmError::Bytecode(err.to_string()))?;
    let code = Bytecode::new_raw(Bytes::from(runtime));

    let verifier = address!("0000000000000000000000000000000000001234");
    let caller = address!("0000000000000000000000000000000000000001");

    let mut db = CacheDB::new(EmptyDB::default());
    db.insert_account_info(
        verifier,
        AccountInfo {
            code_hash: code.hash_slow(),
            code: Some(code),
            ..Default::default()
        },
    );
    db.insert_account_info(
        caller,
        AccountInfo {
            balance: U256::from(u64::MAX),
            ..Default::default()
        },
    );

    let mut evm = Context::mainnet().with_db(db).build_mainnet();
    let tx = TxEnv::builder()
        .caller(caller)
        .kind(TxKind::Call(verifier))
        .data(Bytes::from(data))
        .gas_limit(30_000_000)
        .gas_price(0)
        .build()
        .map_err(|err| EvmError::Execution(format!("{err:?}")))?;

    let outcome = evm
        .transact(tx)
        .map_err(|err| EvmError::Execution(err.to_string()))?;

    match outcome.result {
        ExecutionResult::Success { .. } => Ok(true),
        ExecutionResult::Revert { output, .. } => {
            let selector = &keccak256("ProofInvalid()".as_bytes())[..4];
            if output.len() == 4 && &output[..] == selector {
                Ok(false)
            } else {
                Err(EvmError::Unexpected(format!(
                    "revert 0x{}",
                    hex::encode(&output)
                )))
            }
        }
        ExecutionResult::Halt { reason, .. } => {
            Err(EvmError::Unexpected(format!("halt {reason:?}")))
        }
    }
}

/// Builds the verifier's `verifyProof(bytes,uint256[1])` calldata for `vector`.
fn calldata(vector: &Vector) -> Result<Vec<u8>, EvmError> {
    let [public_input] = vector.public_inputs.as_slice() else {
        return Err(EvmError::Unsupported);
    };
    if vector.proof_system != ProofSystem::Groth16 || vector.primitive != Primitive::Bn254 {
        return Err(EvmError::Unsupported);
    }

    let mut proof = Vec::with_capacity(256);
    proof.extend_from_slice(&g1_be(&vector.proof.a)?);
    proof.extend_from_slice(&g2_be(&vector.proof.b)?);
    proof.extend_from_slice(&g1_be(&vector.proof.c)?);

    let selector = keccak256("verifyProof(bytes,uint256[1])".as_bytes());
    let proof_len = u64::try_from(proof.len())
        .map(U256::from)
        .map_err(|err| EvmError::Execution(err.to_string()))?;

    let mut data = Vec::with_capacity(4 + 32 * 3 + proof.len());
    data.extend_from_slice(&selector[..4]);
    // ABI head: the dynamic `bytes` argument's tail offset, then the inline
    // single-element `uint256[1]` public input.
    data.extend_from_slice(&U256::from(64).to_be_bytes::<32>());
    data.extend_from_slice(&fixed32(public_input)?);
    // ABI tail: the `bytes` length, then the 256-byte proof blob.
    data.extend_from_slice(&proof_len.to_be_bytes::<32>());
    data.extend_from_slice(&proof);
    Ok(data)
}

/// A field element as exactly 32 big-endian bytes, left-padded as needed.
fn fixed32(element: &Element) -> Result<[u8; 32], EvmError> {
    pad_be(element.as_bytes())
}

/// A G1 point as `x || y`, 64 big-endian bytes.
fn g1_be(point: &G1) -> Result<[u8; 64], EvmError> {
    let mut out = [0u8; 64];
    out[..32].copy_from_slice(&fixed32(&point.x)?);
    out[32..].copy_from_slice(&fixed32(&point.y)?);
    Ok(out)
}

/// A G2 point as 128 big-endian bytes, each coordinate imaginary-part-first.
///
/// The vector stores coordinates as `[c0, c1]`; the `alt_bn128` pairing
/// precompile follows the EIP-197 ABI, which places the imaginary part first.
fn g2_be(point: &G2) -> Result<[u8; 128], EvmError> {
    let mut out = [0u8; 128];
    out[..32].copy_from_slice(&fixed32(&point.x[1])?);
    out[32..64].copy_from_slice(&fixed32(&point.x[0])?);
    out[64..96].copy_from_slice(&fixed32(&point.y[1])?);
    out[96..].copy_from_slice(&fixed32(&point.y[0])?);
    Ok(out)
}

fn pad_be(bytes: &[u8]) -> Result<[u8; 32], EvmError> {
    let start = 32usize
        .checked_sub(bytes.len())
        .ok_or(EvmError::ElementTooWide)?;
    let mut out = [0u8; 32];
    out[start..].copy_from_slice(bytes);
    Ok(out)
}
