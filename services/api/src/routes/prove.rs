use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    Json,
};
use serde::Serialize;
use std::sync::Arc;
use std::io::Write;
use std::path::PathBuf;
use tempfile::NamedTempFile;

use crate::AppState;

#[derive(Serialize)]
pub struct ProveResponse {
    pub proof: String,
    pub public_outputs: String,
    pub verify_output: serde_json::Value,
}

/// POST /api/prove — upload a media file, run verifier + SP1 prover (mock mode).
pub async fn prove(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<ProveResponse>, (StatusCode, String)> {
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

    // First verify the file to get the output
    let trust_dir = state.trust_dir.clone();
    let verify_path = tmp_path.clone();
    let verify_output = tokio::task::spawn_blocking(move || {
        verifier::verify(&verify_path, &trust_dir)
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("join: {e}")))?
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("verify: {e}")))?;

    let verify_json = serde_json::to_value(&verify_output)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("serialize: {e}")))?;

    // Create a temp file for JSON sidecar output
    let sidecar = NamedTempFile::with_suffix(".json")
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("tempfile: {e}")))?;
    let sidecar_path = sidecar.path().to_string_lossy().to_string();

    // Shell out to the prover binary
    let prover_dir = state.prover_dir.clone();
    let output = tokio::process::Command::new("cargo")
        .args([
            "run", "--bin", "prove", "--",
            "--file", &tmp_path,
            "--mock",
            "--json-out", &sidecar_path,
        ])
        .current_dir(&prover_dir)
        .output()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("prover spawn: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("prover failed: {stderr}"),
        ));
    }

    // Read the JSON sidecar
    let sidecar_data = tokio::fs::read_to_string(&sidecar_path)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("read sidecar: {e}")))?;
    let sidecar_json: serde_json::Value = serde_json::from_str(&sidecar_data)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("parse sidecar: {e}")))?;

    Ok(Json(ProveResponse {
        proof: sidecar_json["proof"].as_str().unwrap_or("").to_string(),
        public_outputs: sidecar_json["public_outputs"].as_str().unwrap_or("").to_string(),
        verify_output: verify_json,
    }))
}
