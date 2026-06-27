//! EVM parity adapter.
//!
//! Drives audited EVM verifiers in the `revm` in-process EVM and reports whether
//! each accepts its vector. The adapter authors no cryptography; it only marshals
//! the ecosystem-neutral vector into the calldata an audited verifier expects and
//! runs it through the real precompiles.
//!
//! Two statement shapes are supported. A Groth16 BN254 statement runs gnark's
//! MIT, audited Semaphore-lineage Solidity verifier, compiled once to the
//! bytecode committed alongside the vector (see that directory's `generate.sh`),
//! exercising the EIP-196/197 `alt_bn128` precompiles. A BLS signature statement
//! runs the EIP-2537 `BLS12_PAIRING_CHECK` precompile directly under the Prague
//! spec, posing the BLS relation as a pairing product.

use revm::context::result::{ExecutionResult, Output};
use revm::context::{Context, TxEnv};
use revm::database::{CacheDB, EmptyDB};
use revm::primitives::hardfork::SpecId;
use revm::primitives::{Address, Bytes, TxKind, U256, address, keccak256};
use revm::state::{AccountInfo, Bytecode};
use revm::{ExecuteEvm, MainBuilder, MainContext};

use crate::model::{BlsSignature, Element, G1, G2, Groth16, Primitive, Statement, Vector};

/// The audited Groth16 verifier's deployed bytecode, compiled from the committed
/// vector's verifying key (see the vector's `evm/generate.sh` for reproduction).
const VERIFIER_RUNTIME_HEX: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../vectors/bn254-groth16-multiplier/evm/verifier.runtime.hex"
));

/// The BLS12-381 G1 generator, negated.
///
/// The EIP-2537 pairing precompile tests whether a product of pairings is one, so
/// the BLS relation `e(g1, signature) == e(public_key, message_hash)` is posed as
/// `e(-g1, signature) * e(public_key, message_hash) == 1`---the identical form
/// Filecoin's `bls-signatures` verifier evaluates. These are the standard
/// generator's coordinates (fixed by the curve) with the y-coordinate negated,
/// produced by the audited `blstrs` during vector generation; the parity test
/// proves the value agrees with that verifier. See the vector's `PROVENANCE.md`.
const NEG_G1_GENERATOR_X: &str = "17f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb";
const NEG_G1_GENERATOR_Y: &str = "114d1d6855d545a8aa7d76c8cf2e21f267816aef1db507c96655b9d5caac42364e6f38ba0ecb751bad54dcd6b939c2ca";

/// The transaction sender, funded so its calls run; the verdict never depends on it.
const CALLER: Address = address!("0000000000000000000000000000000000000001");
/// The deployment address of the committed Groth16 verifier bytecode.
const VERIFIER: Address = address!("0000000000000000000000000000000000001234");
/// The EIP-2537 `BLS12_PAIRING_CHECK` precompile address.
const PAIRING: Address = address!("000000000000000000000000000000000000000f");

/// A fault in running the EVM adapter, as opposed to a verdict on the proof.
///
/// A rejected proof is reported as `Ok(false)` by [`verify`]; these variants
/// signal that the harness itself could not produce a verdict.
#[derive(Debug, thiserror::Error)]
pub enum EvmError {
    /// The vector is not a shape this adapter's committed verifiers are built for.
    #[error("vector is not a shape the EVM adapter supports")]
    Unsupported,
    /// A field element was wider than its primitive's field encoding.
    #[error("field element exceeds its field encoding width")]
    ElementTooWide,
    /// The committed verifier bytecode could not be decoded.
    #[error("failed to decode the verifier bytecode: {0}")]
    Bytecode(String),
    /// The transaction could not be built or executed.
    #[error("revm execution failed: {0}")]
    Execution(String),
    /// The verifier reverted, halted, or returned output the adapter did not
    /// expect, for a reason other than rejecting the proof.
    #[error("verifier failed unexpectedly: {0}")]
    Unexpected(String),
}

/// Runs the audited EVM verifier over `vector`, returning whether it accepted.
///
/// `Ok(true)` means the on-chain verifier accepted, `Ok(false)` means it rejected
/// (a Groth16 contract reverts `ProofInvalid()`; the pairing precompile returns a
/// zero verdict). An `Err` signals a harness or setup fault, not a verdict on the
/// proof itself.
pub fn verify(vector: &Vector) -> Result<bool, EvmError> {
    match &vector.statement {
        Statement::Groth16(groth16) if vector.primitive == Primitive::Bn254 => {
            verify_groth16(groth16)
        }
        Statement::BlsSignature(bls) if vector.primitive == Primitive::Bls12381 => verify_bls(bls),
        _ => Err(EvmError::Unsupported),
    }
}

/// Runs the committed Groth16 BN254 verifier contract over the statement.
fn verify_groth16(groth16: &Groth16) -> Result<bool, EvmError> {
    let data = groth16_calldata(groth16)?;

    let runtime = hex::decode(VERIFIER_RUNTIME_HEX.trim())
        .map_err(|err| EvmError::Bytecode(err.to_string()))?;
    let code = Bytecode::new_raw(Bytes::from(runtime));

    let mut db = funded_db();
    db.insert_account_info(
        VERIFIER,
        AccountInfo {
            code_hash: code.hash_slow(),
            code: Some(code),
            ..Default::default()
        },
    );

    match transact(db, VERIFIER, data)? {
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

/// Runs the EIP-2537 pairing precompile over the BLS signature relation.
fn verify_bls(bls: &BlsSignature) -> Result<bool, EvmError> {
    let data = pairing_input(bls)?;
    match transact(funded_db(), PAIRING, data)? {
        ExecutionResult::Success {
            output: Output::Call(bytes),
            ..
        } => pairing_verdict(&bytes),
        ExecutionResult::Success { output, .. } => Err(EvmError::Unexpected(format!(
            "unexpected output {output:?}"
        ))),
        ExecutionResult::Revert { output, .. } => Err(EvmError::Unexpected(format!(
            "pairing precompile reverted 0x{}",
            hex::encode(&output)
        ))),
        ExecutionResult::Halt { reason, .. } => {
            Err(EvmError::Unexpected(format!("halt {reason:?}")))
        }
    }
}

/// A `CacheDB` whose sole funded account is the [`CALLER`].
fn funded_db() -> CacheDB<EmptyDB> {
    let mut db = CacheDB::new(EmptyDB::default());
    db.insert_account_info(
        CALLER,
        AccountInfo {
            balance: U256::from(u64::MAX),
            ..Default::default()
        },
    );
    db
}

/// Calls `target` with `data` under the Prague spec, returning the raw result.
fn transact(
    db: CacheDB<EmptyDB>,
    target: Address,
    data: Vec<u8>,
) -> Result<ExecutionResult, EvmError> {
    let mut evm = Context::mainnet()
        .with_db(db)
        .modify_cfg_chained(|cfg| cfg.spec = SpecId::PRAGUE)
        .build_mainnet();
    let tx = TxEnv::builder()
        .caller(CALLER)
        .kind(TxKind::Call(target))
        .data(Bytes::from(data))
        .gas_limit(30_000_000)
        .gas_price(0)
        .build()
        .map_err(|err| EvmError::Execution(format!("{err:?}")))?;

    evm.transact(tx)
        .map(|outcome| outcome.result)
        .map_err(|err| EvmError::Execution(err.to_string()))
}

/// Reads the precompile's 32-byte verdict: a one in the final byte is acceptance.
fn pairing_verdict(output: &[u8]) -> Result<bool, EvmError> {
    match output {
        [head @ .., last]
            if head.len() == 31 && head.iter().all(|byte| *byte == 0) && *last <= 1 =>
        {
            Ok(*last == 1)
        }
        other => Err(EvmError::Unexpected(format!(
            "unexpected pairing verdict 0x{}",
            hex::encode(other)
        ))),
    }
}

/// Builds the verifier's `verifyProof(bytes,uint256[1])` calldata.
fn groth16_calldata(groth16: &Groth16) -> Result<Vec<u8>, EvmError> {
    let [public_input] = groth16.public_inputs.as_slice() else {
        return Err(EvmError::Unsupported);
    };

    let mut proof = Vec::with_capacity(256);
    proof.extend_from_slice(&g1_be(&groth16.proof.a)?);
    proof.extend_from_slice(&g2_be(&groth16.proof.b)?);
    proof.extend_from_slice(&g1_be(&groth16.proof.c)?);

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

/// Builds the EIP-2537 pairing-check input for `e(-g1, sig) * e(pk, H(m)) == 1`,
/// two pairs of a 128-byte G1 and a 256-byte G2 in the precompile's padded ABI.
fn pairing_input(bls: &BlsSignature) -> Result<Vec<u8>, EvmError> {
    let neg_g1_x =
        hex::decode(NEG_G1_GENERATOR_X).map_err(|err| EvmError::Bytecode(err.to_string()))?;
    let neg_g1_y =
        hex::decode(NEG_G1_GENERATOR_Y).map_err(|err| EvmError::Bytecode(err.to_string()))?;

    let mut input = Vec::with_capacity(2 * (128 + 256));
    input.extend_from_slice(&fp48(&neg_g1_x)?);
    input.extend_from_slice(&fp48(&neg_g1_y)?);
    input.extend_from_slice(&g2_padded(&bls.signature)?);
    input.extend_from_slice(&g1_padded(&bls.public_key)?);
    input.extend_from_slice(&g2_padded(&bls.message_hash)?);
    Ok(input)
}

/// A field element as exactly 32 big-endian bytes, left-padded as needed.
fn fixed32(element: &Element) -> Result<[u8; 32], EvmError> {
    pad_be::<32>(element.as_bytes())
}

/// A G1 point as `x || y`, 64 big-endian bytes (BN254 width).
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

/// A BLS12-381 field element as a 64-byte EIP-2537 field slot: a 48-byte
/// big-endian value left-padded with 16 zero bytes.
fn fp48(bytes: &[u8]) -> Result<[u8; 64], EvmError> {
    pad_be::<64>(bytes)
}

/// A BLS12-381 G1 point in the precompile's 128-byte padded form, `x || y`.
fn g1_padded(point: &G1) -> Result<[u8; 128], EvmError> {
    let mut out = [0u8; 128];
    out[..64].copy_from_slice(&fp48(point.x.as_bytes())?);
    out[64..].copy_from_slice(&fp48(point.y.as_bytes())?);
    Ok(out)
}

/// A BLS12-381 G2 point in the precompile's 256-byte padded form.
///
/// EIP-2537 orders the quadratic-extension coordinates `c0` then `c1` (the real
/// part first), the vector's own `[c0, c1]` order, so no reordering is applied.
fn g2_padded(point: &G2) -> Result<[u8; 256], EvmError> {
    let mut out = [0u8; 256];
    out[..64].copy_from_slice(&fp48(point.x[0].as_bytes())?);
    out[64..128].copy_from_slice(&fp48(point.x[1].as_bytes())?);
    out[128..192].copy_from_slice(&fp48(point.y[0].as_bytes())?);
    out[192..].copy_from_slice(&fp48(point.y[1].as_bytes())?);
    Ok(out)
}

/// Right-aligns `bytes` into an `N`-byte buffer, erroring if they do not fit.
fn pad_be<const N: usize>(bytes: &[u8]) -> Result<[u8; N], EvmError> {
    let start = N.checked_sub(bytes.len()).ok_or(EvmError::ElementTooWide)?;
    let mut out = [0u8; N];
    out[start..].copy_from_slice(bytes);
    Ok(out)
}
