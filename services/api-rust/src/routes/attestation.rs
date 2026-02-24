use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use borsh::BorshDeserialize;
use serde::Serialize;
use solana_rpc_client::rpc_client::RpcClient;
use solana_rpc_client_api::config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use solana_rpc_client_api::filter::{Memcmp, RpcFilterType};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::Arc;

use crate::AppState;

const ATTESTATION_SEED: &[u8] = b"attestation";

/// Anchor account discriminator for Attestation
const ATTESTATION_DISCRIMINATOR: [u8; 8] = [152, 125, 183, 86, 36, 146, 121, 73];

/// Anchor account discriminator for IdentityAttestation
const IDENTITY_DISCRIMINATOR: [u8; 8] = [151, 136, 164, 76, 84, 171, 65, 139];

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
    cert_fingerprint: String,
    submitted_by: [u8; 32], // Pubkey as raw bytes
    timestamp: i64,
    #[allow(dead_code)]
    bump: u8,
    proof_type: String,
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
    pub cert_fingerprint: String,
    pub submitted_by: String,
    pub timestamp: i64,
    pub proof_type: String,
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

        let mut cursor = std::io::Cursor::new(&data[8..]);
        let attestation = AttestationAccount::deserialize_reader(&mut cursor)
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
            cert_fingerprint: att.cert_fingerprint,
            submitted_by: Pubkey::from(att.submitted_by).to_string(),
            timestamp: att.timestamp,
            proof_type: att.proof_type,
        })),
        None => Err((StatusCode::NOT_FOUND, "attestation not found".to_string())),
    }
}

// ── List all attestations ────────────────────────────────────────────

#[derive(BorshDeserialize)]
struct IdentityAttestationAccount {
    content_hash: [u8; 32],
    domain: String,
    #[allow(dead_code)]
    email_hash: [u8; 32],
    submitted_by: [u8; 32],
    timestamp: i64,
    #[allow(dead_code)]
    bump: u8,
    proof_type: String,
}

#[derive(Serialize)]
pub struct AttestationListItem {
    pub content_hash: String,
    pub proof_type: String,
    pub timestamp: i64,
    pub kind: String, // "c2pa" or "identity"
    // C2PA fields (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issuer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trust_list_match: Option<String>,
    // Identity fields (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
}

/// GET /api/attestations — list all attestations (C2PA + identity).
pub async fn list_all(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<AttestationListItem>>, (StatusCode, String)> {
    let program_id = Pubkey::from_str(&state.program_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("bad program id: {e}")))?;

    let rpc_url = state.rpc_url.clone();

    let result = tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<AttestationListItem>> {
        let client = RpcClient::new(&rpc_url);
        let mut items = Vec::new();

        // Fetch C2PA attestations
        let c2pa_filter = RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            0,
            ATTESTATION_DISCRIMINATOR.to_vec(),
        ));
        let config = RpcProgramAccountsConfig {
            filters: Some(vec![c2pa_filter]),
            account_config: RpcAccountInfoConfig {
                commitment: Some(CommitmentConfig::confirmed()),
                ..Default::default()
            },
            ..Default::default()
        };
        if let Ok(accounts) = client.get_program_accounts_with_config(&program_id, config) {
            for (_pubkey, account) in accounts {
                if account.data.len() < 8 { continue; }
                let mut cursor = std::io::Cursor::new(&account.data[8..]);
                if let Ok(att) = AttestationAccount::deserialize_reader(&mut cursor) {
                    items.push(AttestationListItem {
                        content_hash: hex::encode(att.content_hash),
                        proof_type: att.proof_type,
                        timestamp: att.timestamp,
                        kind: "c2pa".to_string(),
                        issuer: if att.issuer.is_empty() { None } else { Some(att.issuer) },
                        trust_list_match: if att.trust_list_match.is_empty() { None } else { Some(att.trust_list_match) },
                        domain: None,
                    });
                }
            }
        }

        // Fetch identity attestations
        let id_filter = RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            0,
            IDENTITY_DISCRIMINATOR.to_vec(),
        ));
        let config = RpcProgramAccountsConfig {
            filters: Some(vec![id_filter]),
            account_config: RpcAccountInfoConfig {
                commitment: Some(CommitmentConfig::confirmed()),
                ..Default::default()
            },
            ..Default::default()
        };
        if let Ok(accounts) = client.get_program_accounts_with_config(&program_id, config) {
            for (_pubkey, account) in accounts {
                if account.data.len() < 8 { continue; }
                let mut cursor = std::io::Cursor::new(&account.data[8..]);
                if let Ok(att) = IdentityAttestationAccount::deserialize_reader(&mut cursor) {
                    items.push(AttestationListItem {
                        content_hash: hex::encode(att.content_hash),
                        proof_type: att.proof_type,
                        timestamp: att.timestamp,
                        kind: "identity".to_string(),
                        issuer: None,
                        trust_list_match: None,
                        domain: Some(att.domain),
                    });
                }
            }
        }

        // Sort by timestamp descending (newest first)
        items.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(items)
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("join: {e}")))?
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("rpc: {e}")))?;

    Ok(Json(result))
}
