use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::Html,
    Json,
};
use borsh::BorshSerialize;
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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
use std::time::{Duration, Instant};
use uuid::Uuid;

use crate::{AppState, VerificationEntry};

const IDENTITY_SEED: &[u8] = b"identity";

/// Anchor discriminator for submit_identity: sha256("global:submit_identity")[..8]
const SUBMIT_IDENTITY_DISCRIMINATOR: [u8; 8] = [72, 233, 38, 193, 138, 137, 49, 75];

/// Verification entries expire after 30 minutes.
const EXPIRY: Duration = Duration::from_secs(30 * 60);

// ── POST /api/identity/start ─────────────────────────────────────────

#[derive(Serialize)]
pub struct StartResponse {
    pub token: String,
    pub content_hash: String,
    pub domain: String,
    pub verification_url: Option<String>,
}

pub async fn start(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<StartResponse>, (StatusCode, String)> {
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut email: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("multipart error: {e}")))?
    {
        match field.name() {
            Some("file") => {
                let data = field
                    .bytes()
                    .await
                    .map_err(|e| (StatusCode::BAD_REQUEST, format!("read file: {e}")))?;
                file_bytes = Some(data.to_vec());
            }
            Some("email") => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| (StatusCode::BAD_REQUEST, format!("read email: {e}")))?;
                email = Some(text);
            }
            _ => {}
        }
    }

    let file_bytes =
        file_bytes.ok_or_else(|| (StatusCode::BAD_REQUEST, "missing file field".to_string()))?;
    let email =
        email.ok_or_else(|| (StatusCode::BAD_REQUEST, "missing email field".to_string()))?;

    // Validate email
    let at_pos = email
        .find('@')
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "invalid email: no @".to_string()))?;
    let domain = &email[at_pos + 1..];
    if !domain.contains('.') || domain.len() < 3 {
        return Err((StatusCode::BAD_REQUEST, "invalid email domain".to_string()));
    }
    let domain = domain.to_lowercase();

    // Compute content hash
    let content_hash = hex::encode(Sha256::digest(&file_bytes));

    // Generate token
    let token = Uuid::new_v4().to_string();

    // Clean up expired entries, then insert
    {
        let mut map = state.verifications.lock().unwrap();
        let cutoff = Instant::now() - EXPIRY;
        map.retain(|_, entry| entry.created_at > cutoff);
        map.insert(
            token.clone(),
            VerificationEntry {
                email: email.clone(),
                domain: domain.clone(),
                content_hash: content_hash.clone(),
                verified: false,
                created_at: Instant::now(),
            },
        );
    }

    let base = std::env::var("PUBLIC_URL")
        .unwrap_or_else(|_| "http://localhost:3001".to_string());
    let verify_url = format!("{base}/api/identity/verify/{token}");

    // Send verification email via SMTP (or fall back to dev mode)
    let smtp_host = std::env::var("SMTP_HOST").ok();
    let verification_url = if let Some(host) = smtp_host {
        let smtp_user = std::env::var("SMTP_USER")
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "SMTP_USER not set".to_string()))?;
        let smtp_pass = std::env::var("SMTP_PASS")
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "SMTP_PASS not set".to_string()))?;
        let from_addr = std::env::var("SMTP_FROM")
            .unwrap_or_else(|_| smtp_user.clone());

        let email_body = format!(
            r#"<!DOCTYPE html>
<html><head><meta charset="utf-8"></head>
<body style="font-family:system-ui,sans-serif;background:#0a0a0f;color:#e5e5e5;margin:0;padding:40px;">
<div style="max-width:480px;margin:0 auto;background:#1a1a2e;border:1px solid #2d2d44;border-radius:12px;padding:40px;text-align:center;">
<h1 style="color:#facc15;margin:0 0 8px;font-size:24px;">R3L Provenance</h1>
<p style="color:#9ca3af;margin:0 0 24px;">Verify your email domain for media attestation.</p>
<p style="color:#e5e5e5;margin:0 0 24px;">Click below to confirm you own <strong style="color:#facc15;">{domain}</strong></p>
<a href="{verify_url}" style="display:inline-block;background:#ca8a04;color:#fff;text-decoration:none;padding:14px 32px;border-radius:8px;font-weight:600;font-size:16px;">Verify Email</a>
<p style="color:#6b7280;font-size:12px;margin:24px 0 0;">This link expires in 30 minutes.</p>
</div>
</body></html>"#
        );

        let email_msg = Message::builder()
            .from(from_addr.parse().map_err(|e| {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("bad from addr: {e}"))
            })?)
            .to(email.parse().map_err(|e| {
                (StatusCode::BAD_REQUEST, format!("bad email addr: {e}"))
            })?)
            .subject("R3L — Verify your email domain")
            .header(ContentType::TEXT_HTML)
            .body(email_body)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("build email: {e}")))?;

        let creds = Credentials::new(smtp_user, smtp_pass);

        let mailer = SmtpTransport::relay(&host)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("smtp relay: {e}")))?
            .credentials(creds)
            .build();

        mailer.send(&email_msg).map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("send email: {e}"))
        })?;

        tracing::info!("Verification email sent to {email} for domain {domain}");
        None
    } else {
        tracing::warn!("SMTP_HOST not set — returning verification URL directly (dev mode)");
        Some(verify_url)
    };

    Ok(Json(StartResponse {
        token,
        content_hash,
        domain,
        verification_url,
    }))
}

// ── GET /api/identity/verify/{token} ─────────────────────────────────

pub async fn verify_email(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
) -> Result<Html<String>, (StatusCode, String)> {
    let mut map = state.verifications.lock().unwrap();

    let entry = map
        .get_mut(&token)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "verification not found".to_string()))?;

    if entry.created_at.elapsed() > EXPIRY {
        return Err((StatusCode::GONE, "verification expired".to_string()));
    }

    entry.verified = true;
    let domain = entry.domain.clone();

    Ok(Html(format!(
        r#"<!DOCTYPE html>
<html><head><title>R3L - Email Verified</title>
<style>
body {{ font-family: system-ui; background: #0a0a0f; color: #e5e5e5; display: flex; align-items: center; justify-content: center; height: 100vh; margin: 0; }}
.card {{ background: #1a1a2e; border: 1px solid #2d2d44; border-radius: 12px; padding: 40px; text-align: center; max-width: 400px; }}
.check {{ font-size: 48px; margin-bottom: 16px; }}
h1 {{ color: #4ade80; margin: 0 0 8px; font-size: 24px; }}
p {{ color: #9ca3af; margin: 0; }}
.domain {{ color: #facc15; font-weight: 600; }}
</style></head>
<body><div class="card">
<div class="check">&#10003;</div>
<h1>Email Verified</h1>
<p>Domain <span class="domain">{domain}</span> confirmed.</p>
<p style="margin-top: 16px;">You can close this tab and return to R3L.</p>
</div></body></html>"#
    )))
}

// ── GET /api/identity/status/{token} ─────────────────────────────────

#[derive(Serialize)]
pub struct StatusResponse {
    pub verified: bool,
    pub domain: String,
    pub content_hash: String,
    pub expired: bool,
}

pub async fn status(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
) -> Result<Json<StatusResponse>, (StatusCode, String)> {
    let map = state.verifications.lock().unwrap();

    let entry = map
        .get(&token)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "verification not found".to_string()))?;

    let expired = entry.created_at.elapsed() > EXPIRY;

    Ok(Json(StatusResponse {
        verified: entry.verified && !expired,
        domain: entry.domain.clone(),
        content_hash: entry.content_hash.clone(),
        expired,
    }))
}

// ── POST /api/identity/attest ────────────────────────────────────────

#[derive(Deserialize)]
pub struct AttestRequest {
    pub token: String,
}

#[derive(Serialize)]
pub struct IdentityAttestResponse {
    pub signature: String,
    pub identity_pda: String,
    pub content_hash: String,
    pub domain: String,
}

/// Borsh-encode the submit_identity instruction data.
///
/// Layout: discriminator(8) + content_hash([u8;32]) + domain(Borsh String) + email_hash([u8;32])
fn encode_identity_data(content_hash: &[u8; 32], domain: &str, email_hash: &[u8; 32]) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&SUBMIT_IDENTITY_DISCRIMINATOR);
    data.extend_from_slice(content_hash);
    BorshSerialize::serialize(&domain.to_string(), &mut data).unwrap();
    data.extend_from_slice(email_hash);
    data
}

pub async fn attest_identity(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AttestRequest>,
) -> Result<Json<IdentityAttestResponse>, (StatusCode, String)> {
    // 1. Look up and validate verification
    let (email, domain, content_hash_hex) = {
        let map = state.verifications.lock().unwrap();
        let entry = map.get(&req.token).ok_or_else(|| {
            (StatusCode::NOT_FOUND, "verification not found".to_string())
        })?;
        if !entry.verified {
            return Err((
                StatusCode::BAD_REQUEST,
                "email not yet verified".to_string(),
            ));
        }
        if entry.created_at.elapsed() > EXPIRY {
            return Err((StatusCode::GONE, "verification expired".to_string()));
        }
        (
            entry.email.clone(),
            entry.domain.clone(),
            entry.content_hash.clone(),
        )
    };

    // 2. Compute hashes
    let content_hash_bytes = hex::decode(&content_hash_hex)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("hex: {e}")))?;
    let mut content_hash = [0u8; 32];
    content_hash.copy_from_slice(&content_hash_bytes);

    let email_hash: [u8; 32] = Sha256::digest(email.as_bytes()).into();

    // 3. Build Solana transaction
    let program_id = Pubkey::from_str(&state.program_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("bad program id: {e}")))?;

    let (identity_pda, _bump) = Pubkey::find_program_address(
        &[IDENTITY_SEED, &content_hash, domain.as_bytes()],
        &program_id,
    );

    let keypair_path = state.keypair_path.clone();
    let rpc_url = state.rpc_url.clone();
    let domain_clone = domain.clone();

    let result = tokio::task::spawn_blocking(move || -> anyhow::Result<(String, String)> {
        let payer = read_keypair_file(&keypair_path)
            .map_err(|e| anyhow::anyhow!("read keypair: {e}"))?;
        let client = RpcClient::new(&rpc_url);

        let ix_data = encode_identity_data(&content_hash, &domain_clone, &email_hash);

        let accounts = vec![
            AccountMeta::new(identity_pda, false),
            AccountMeta::new(payer.pubkey(), true),
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
        Ok((sig.to_string(), identity_pda.to_string()))
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("join: {e}")))?
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("submit: {e}")))?;

    Ok(Json(IdentityAttestResponse {
        signature: result.0,
        identity_pda: result.1,
        content_hash: content_hash_hex,
        domain,
    }))
}
