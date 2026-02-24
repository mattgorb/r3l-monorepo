use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    Json,
};
use borsh::BorshSerialize;
use serde::Serialize;
use solana_rpc_client::rpc_client::RpcClient;
#[allow(deprecated)]
use solana_sdk::system_program;
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::read_keypair_file,
    signer::Signer,
    transaction::Transaction,
};
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use tempfile::NamedTempFile;

use crate::AppState;

const ATTESTATION_SEED: &[u8] = b"attestation";

/// Anchor discriminator for submit_attestation: sha256("global:submit_attestation")[..8]
const SUBMIT_ATTESTATION_DISCRIMINATOR: [u8; 8] = [238, 220, 255, 105, 183, 211, 40, 83];

#[derive(Serialize)]
pub struct AttestResponse {
    pub signature: String,
    pub attestation_pda: String,
    pub content_hash: String,
    pub verify_output: serde_json::Value,
}

/// Borsh-encode the submit_attestation instruction data.
///
/// Layout: discriminator(8) + content_hash([u8;32]) + has_c2pa(bool) +
///   8 Borsh Strings (4-byte LE length + utf8)
fn encode_attestation_data(
    content_hash: &[u8; 32],
    has_c2pa: bool,
    trust_list_match: &str,
    validation_state: &str,
    digital_source_type: &str,
    issuer: &str,
    common_name: &str,
    software_agent: &str,
    signing_time: &str,
    cert_fingerprint: &str,
) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&SUBMIT_ATTESTATION_DISCRIMINATOR);

    // content_hash: [u8; 32] — raw bytes
    data.extend_from_slice(content_hash);

    // has_c2pa: bool
    BorshSerialize::serialize(&has_c2pa, &mut data).unwrap();

    // 8 String fields (Borsh: 4-byte LE length + utf8 bytes)
    for s in [
        trust_list_match,
        validation_state,
        digital_source_type,
        issuer,
        common_name,
        software_agent,
        signing_time,
        cert_fingerprint,
    ] {
        BorshSerialize::serialize(&s.to_string(), &mut data).unwrap();
    }

    data
}

/// POST /api/attest — verify a file and submit attestation to Solana in one step.
/// No ZK proof needed — the server acts as a trusted verifier.
pub async fn attest(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<AttestResponse>, (StatusCode, String)> {
    // 1. Extract uploaded file
    let field = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("multipart error: {e}")))?
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "no file field".to_string()))?;

    let original_name = field.file_name().unwrap_or("upload").to_string();
    let data = field
        .bytes()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("read error: {e}")))?;

    let extension = PathBuf::from(&original_name)
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();

    let mut tmp = NamedTempFile::with_suffix(&extension)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("tempfile: {e}")))?;
    tmp.write_all(&data)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("write: {e}")))?;

    let tmp_path = tmp.path().to_string_lossy().to_string();
    let trust_dir = state.trust_dir.clone();

    // 2. Verify the file
    let verify_output = tokio::task::spawn_blocking(move || {
        verifier::verify(&tmp_path, &trust_dir)
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("join: {e}")))?
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("verify: {e:#}")))?;

    let verify_json = serde_json::to_value(&verify_output)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("serialize: {e}")))?;

    // 3. Extract content_hash
    let content_hash_hex = verify_output
        .content_hash
        .as_ref()
        .ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, "no content hash".to_string()))?
        .clone();
    let content_hash_bytes = hex::decode(&content_hash_hex)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("hex: {e}")))?;
    let mut content_hash = [0u8; 32];
    content_hash.copy_from_slice(&content_hash_bytes);

    // 4. Build and send Solana transaction
    let program_id = Pubkey::from_str(&state.program_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("bad program id: {e}")))?;

    let (attestation_pda, _bump) = Pubkey::find_program_address(
        &[ATTESTATION_SEED, &content_hash],
        &program_id,
    );

    let keypair_path = state.keypair_path.clone();
    let rpc_url = state.rpc_url.clone();

    let has_c2pa = verify_output.has_c2pa;
    let trust_list_match = verify_output.trust_list_match.unwrap_or_default();
    let validation_state = verify_output.validation_state.unwrap_or_default();
    let digital_source_type = verify_output.digital_source_type.unwrap_or_default();
    let issuer_val = verify_output.issuer.unwrap_or_default();
    let common_name_val = verify_output.common_name.unwrap_or_default();
    let software_agent = verify_output.software_agent.unwrap_or_default();
    let signing_time = verify_output.signing_time.unwrap_or_default();
    let cert_fingerprint = String::new(); // TODO: extract from verifier

    let result = tokio::task::spawn_blocking(move || -> anyhow::Result<(String, String)> {
        let payer = read_keypair_file(&keypair_path)
            .map_err(|e| anyhow::anyhow!("read keypair: {e}"))?;
        let client = RpcClient::new(&rpc_url);

        let ix_data = encode_attestation_data(
            &content_hash,
            has_c2pa,
            &trust_list_match,
            &validation_state,
            &digital_source_type,
            &issuer_val,
            &common_name_val,
            &software_agent,
            &signing_time,
            &cert_fingerprint,
        );

        let accounts = vec![
            AccountMeta::new(attestation_pda, false),
            AccountMeta::new(payer.pubkey(), true), // authority (signer + payer)
            AccountMeta::new_readonly(system_program::id(), false),
        ];

        let ix = Instruction::new_with_bytes(program_id, &ix_data, accounts);
        let compute_ix = ComputeBudgetInstruction::set_compute_unit_limit(200_000);

        let recent_hash = client.get_latest_blockhash()?;
        let tx = Transaction::new_signed_with_payer(
            &[compute_ix, ix],
            Some(&payer.pubkey()),
            &[&payer],
            recent_hash,
        );

        let sig = client.send_and_confirm_transaction(&tx)?;
        Ok((sig.to_string(), attestation_pda.to_string()))
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("join: {e}")))?
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("submit: {e}")))?;

    Ok(Json(AttestResponse {
        signature: result.0,
        attestation_pda: result.1,
        content_hash: content_hash_hex,
        verify_output: verify_json,
    }))
}
