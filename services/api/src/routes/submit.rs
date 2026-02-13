use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use borsh::BorshSerialize;
use serde::{Deserialize, Serialize};
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
use std::str::FromStr;
use std::sync::Arc;

use crate::AppState;

const ATTESTATION_SEED: &[u8] = b"attestation";

/// Anchor discriminator for submit_proof: sha256("global:submit_proof")[..8]
const SUBMIT_PROOF_DISCRIMINATOR: [u8; 8] = [54, 241, 46, 84, 4, 212, 46, 94];

#[derive(Deserialize)]
pub struct SubmitRequest {
    pub content_hash: String,
    pub has_c2pa: bool,
    pub trust_list_match: String,
    pub validation_state: String,
    pub digital_source_type: String,
    pub issuer: String,
    pub common_name: String,
    pub software_agent: String,
    pub signing_time: String,
    #[serde(default)]
    pub proof: Option<String>,
    #[serde(default)]
    pub public_inputs: Option<String>,
}

#[derive(Serialize)]
pub struct SubmitResponse {
    pub signature: String,
    pub attestation_pda: String,
}

/// Borsh-serialize the submit_proof instruction data.
/// Layout: discriminator + proof + public_inputs + content_hash + has_c2pa + strings...
fn encode_instruction_data(req: &SubmitRequest, content_hash: &[u8; 32]) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&SUBMIT_PROOF_DISCRIMINATOR);

    let proof_bytes = req.proof.as_deref()
        .and_then(|s| hex::decode(s).ok())
        .unwrap_or_default();
    let public_inputs_bytes = req.public_inputs.as_deref()
        .and_then(|s| hex::decode(s).ok())
        .unwrap_or_default();

    // Vec<u8> — 4-byte LE length + bytes
    BorshSerialize::serialize(&proof_bytes, &mut data).unwrap();
    BorshSerialize::serialize(&public_inputs_bytes, &mut data).unwrap();

    // [u8; 32] — fixed 32 bytes
    data.extend_from_slice(content_hash);

    // bool — 1 byte
    BorshSerialize::serialize(&req.has_c2pa, &mut data).unwrap();

    // Strings — 4-byte LE length + UTF-8 bytes
    BorshSerialize::serialize(&req.trust_list_match, &mut data).unwrap();
    BorshSerialize::serialize(&req.validation_state, &mut data).unwrap();
    BorshSerialize::serialize(&req.digital_source_type, &mut data).unwrap();
    BorshSerialize::serialize(&req.issuer, &mut data).unwrap();
    BorshSerialize::serialize(&req.common_name, &mut data).unwrap();
    BorshSerialize::serialize(&req.software_agent, &mut data).unwrap();
    BorshSerialize::serialize(&req.signing_time, &mut data).unwrap();

    data
}

/// POST /api/submit — submit an attestation to Solana.
pub async fn submit(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SubmitRequest>,
) -> Result<Json<SubmitResponse>, (StatusCode, String)> {
    let content_hash_bytes = hex::decode(&req.content_hash)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid content_hash hex: {e}")))?;
    if content_hash_bytes.len() != 32 {
        return Err((StatusCode::BAD_REQUEST, "content_hash must be 32 bytes".to_string()));
    }
    let mut content_hash = [0u8; 32];
    content_hash.copy_from_slice(&content_hash_bytes);

    let program_id = Pubkey::from_str(&state.program_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("bad program id: {e}")))?;

    let (attestation_pda, _bump) = Pubkey::find_program_address(
        &[ATTESTATION_SEED, &content_hash],
        &program_id,
    );

    let keypair_path = state.keypair_path.clone();
    let rpc_url = state.rpc_url.clone();

    let result = tokio::task::spawn_blocking(move || -> anyhow::Result<(String, String)> {
        let payer = read_keypair_file(&keypair_path)
            .map_err(|e| anyhow::anyhow!("read keypair: {e}"))?;
        let client = RpcClient::new(&rpc_url);

        let ix_data = encode_instruction_data(&req, &content_hash);

        let accounts = vec![
            AccountMeta::new(attestation_pda, false),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(system_program::id(), false),
        ];

        let ix = Instruction::new_with_bytes(program_id, &ix_data, accounts);
        let compute_ix = ComputeBudgetInstruction::set_compute_unit_limit(400_000);

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

    Ok(Json(SubmitResponse {
        signature: result.0,
        attestation_pda: result.1,
    }))
}
