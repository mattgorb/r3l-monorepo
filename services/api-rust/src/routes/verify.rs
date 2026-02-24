use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use tempfile::NamedTempFile;
use std::io::Write;
use std::path::PathBuf;

use crate::AppState;

/// POST /api/verify — upload a media file and get C2PA verification results.
pub async fn verify(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Extract the uploaded file
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

    // Preserve original extension (c2pa-rs needs it for format detection)
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

    // verifier::verify is sync + blocking — run on blocking thread pool
    let result = tokio::task::spawn_blocking(move || {
        verifier::verify(&tmp_path, &trust_dir)
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("join: {e}")))?;

    match result {
        Ok(output) => {
            let json = serde_json::to_value(&output)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("serialize: {e}")))?;
            Ok(Json(json))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, format!("verify: {e:#}"))),
    }
}
