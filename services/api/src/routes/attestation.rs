use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use borsh::BorshDeserialize;
use serde::Serialize;
use solana_rpc_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::Arc;

use crate::AppState;

const ATTESTATION_SEED: &[u8] = b"attestation";

/// Anchor account discriminator for Attestation
const ATTESTATION_DISCRIMINATOR: [u8; 8] = [152, 125, 183, 86, 36, 146, 121, 73];

/// On-chain Attestation account (borsh-deserializable).
/// Must match the Anchor program's state.rs exactly.
#[derive(BorshDeserialize)]
struct AttestationAccount {
    content_hash: [u8; 32],
    has_c2pa: bool,
    trust_list_match: String,
    validation_state: String,
    digital_source_type: String,
    issuer: String,
    common_name: String,
    software_agent: String,
    signing_time: String,
    submitted_by: [u8; 32], // Pubkey as raw bytes
    timestamp: i64,
    #[allow(dead_code)]
    bump: u8,
}

#[derive(Serialize)]
pub struct AttestationResponse {
    pub content_hash: String,
    pub has_c2pa: bool,
    pub trust_list_match: String,
    pub validation_state: String,
    pub digital_source_type: String,
    pub issuer: String,
    pub common_name: String,
    pub software_agent: String,
    pub signing_time: String,
    pub submitted_by: String,
    pub timestamp: i64,
}

/// GET /api/attestation/:hash — look up an attestation by content hash.
pub async fn lookup(
    State(state): State<Arc<AppState>>,
    Path(hash): Path<String>,
) -> Result<Json<AttestationResponse>, (StatusCode, String)> {
    let content_hash_bytes = hex::decode(&hash)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid hex: {e}")))?;
    if content_hash_bytes.len() != 32 {
        return Err((StatusCode::BAD_REQUEST, "hash must be 32 bytes hex".to_string()));
    }

    let program_id = Pubkey::from_str(&state.program_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("bad program id: {e}")))?;

    let (pda, _) = Pubkey::find_program_address(
        &[ATTESTATION_SEED, &content_hash_bytes],
        &program_id,
    );

    let rpc_url = state.rpc_url.clone();
    let result = tokio::task::spawn_blocking(move || -> anyhow::Result<Option<AttestationAccount>> {
        let client = RpcClient::new(&rpc_url);
        let account = match client.get_account(&pda) {
            Ok(a) => a,
            Err(_) => return Ok(None),
        };

        let data = &account.data;
        if data.len() < 8 {
            return Ok(None);
        }

        // Verify discriminator
        if data[..8] != ATTESTATION_DISCRIMINATOR {
            return Ok(None);
        }

        let attestation = AttestationAccount::try_from_slice(&data[8..])
            .map_err(|e| anyhow::anyhow!("deserialize: {e}"))?;
        Ok(Some(attestation))
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("join: {e}")))?
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("rpc: {e}")))?;

    match result {
        Some(att) => Ok(Json(AttestationResponse {
            content_hash: hex::encode(att.content_hash),
            has_c2pa: att.has_c2pa,
            trust_list_match: att.trust_list_match,
            validation_state: att.validation_state,
            digital_source_type: att.digital_source_type,
            issuer: att.issuer,
            common_name: att.common_name,
            software_agent: att.software_agent,
            signing_time: att.signing_time,
            submitted_by: Pubkey::from(att.submitted_by).to_string(),
            timestamp: att.timestamp,
        })),
        None => Err((StatusCode::NOT_FOUND, "attestation not found".to_string())),
    }
}
