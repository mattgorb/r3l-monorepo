use anyhow::{Context as AnyhowContext, Result};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

const DEFAULT_TRUST_DIR: &str = "/data/trust";

#[derive(Serialize)]
pub struct VerifyOutput {
    pub path: String,
    pub content_hash: Option<String>,
    pub has_c2pa: bool,
    pub trust_list_match: Option<String>,
    pub validation_state: Option<String>,
    pub validation_error_count: Option<usize>,
    pub validation_codes: Option<Vec<String>>,
    pub title: Option<String>,
    pub format: Option<String>,
    pub digital_source_type: Option<String>,
    pub claim_generator: Option<String>,
    pub software_agent: Option<String>,
    pub issuer: Option<String>,
    pub common_name: Option<String>,
    pub signing_time: Option<String>,
    pub sig_algorithm: Option<String>,
    pub actions: Option<Value>,
    pub ingredients: Option<Value>,
    pub manifest_store: Option<Value>,
    pub error: Option<String>,
}

impl VerifyOutput {
    fn unsigned(path: String, content_hash: Option<String>) -> Self {
        Self {
            path,
            content_hash,
            has_c2pa: false,
            trust_list_match: None,
            validation_state: None,
            validation_error_count: None,
            validation_codes: None,
            title: None,
            format: None,
            digital_source_type: None,
            claim_generator: None,
            software_agent: None,
            issuer: None,
            common_name: None,
            signing_time: None,
            sig_algorithm: None,
            actions: None,
            ingredients: None,
            manifest_store: None,
            error: None,
        }
    }

    pub fn with_error(path: String, error: String) -> Self {
        let mut o = Self::unsigned(path, None);
        o.error = Some(error);
        o
    }
}

/// Verify a file's C2PA provenance and return structured output.
pub fn verify(path: &str, trust_dir: &str) -> Result<VerifyOutput> {
    anyhow::ensure!(Path::new(path).exists(), "File not found: {}", path);

    // Compute content hash (SHA-256 of file bytes)
    let file_bytes = fs::read(path)
        .with_context(|| format!("reading file: {path}"))?;
    let content_hash = Some(hex::encode(Sha256::digest(&file_bytes)));

    let trust_path = Path::new(trust_dir);
    let official_pem = load_pems(&trust_path.join("official"))?;
    let curated_pem = load_pems(&trust_path.join("curated"))?;

    let (reader, trust_list_match) = match resolve_trust(path, &official_pem, &curated_pem)? {
        None => return Ok(VerifyOutput::unsigned(path.to_string(), content_hash)),
        Some(pair) => pair,
    };

    let has_c2pa = reader.active_manifest().is_some();
    let validation_state = Some(format!("{:?}", reader.validation_state()));
    let validation_error_count = reader.validation_status().map(|arr| arr.len());
    let validation_codes = reader
        .validation_status()
        .map(|arr| arr.iter().map(|s| s.code().to_string()).collect());

    let manifest_store = if has_c2pa {
        Some(serde_json::from_str::<Value>(&reader.json())?)
    } else {
        None
    };

    let props = manifest_store
        .as_ref()
        .map(extract_props)
        .unwrap_or_default();

    Ok(VerifyOutput {
        path: path.to_string(),
        content_hash,
        has_c2pa,
        trust_list_match: Some(trust_list_match),
        validation_state,
        validation_error_count,
        validation_codes,
        title: props.title,
        format: props.format,
        digital_source_type: props.digital_source_type,
        claim_generator: props.claim_generator,
        software_agent: props.software_agent,
        issuer: props.issuer,
        common_name: props.common_name,
        signing_time: props.signing_time,
        sig_algorithm: props.sig_algorithm,
        actions: props.actions,
        ingredients: props.ingredients,
        manifest_store,
        error: None,
    })
}

/// Convenience: verify using the default or TRUST_DIR env var.
pub fn verify_with_env(path: &str) -> Result<VerifyOutput> {
    let trust_dir = std::env::var("TRUST_DIR").unwrap_or_else(|_| DEFAULT_TRUST_DIR.to_string());
    verify(path, &trust_dir)
}

/// Load and concatenate all .pem files from a directory.
fn load_pems(dir: &Path) -> Result<String> {
    let mut combined = String::new();
    if !dir.exists() {
        return Ok(combined);
    }
    let mut entries: Vec<_> = fs::read_dir(dir)
        .with_context(|| format!("reading trust dir: {}", dir.display()))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "pem"))
        .collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let content = fs::read_to_string(entry.path())
            .with_context(|| format!("reading PEM: {}", entry.path().display()))?;
        combined.push_str(&content);
        if !combined.ends_with('\n') {
            combined.push('\n');
        }
    }
    Ok(combined)
}

/// Try to open a C2PA file with trust anchors. Returns None if unsigned.
fn try_read(path: &str, trust_pem: &str) -> Result<Option<c2pa::Reader>> {
    let result = if trust_pem.is_empty() {
        c2pa::Reader::from_file(path)
    } else {
        let settings = c2pa::settings::Settings::new()
            .with_value("trust.trust_anchors", trust_pem)
            .map_err(|e| anyhow::anyhow!("settings: {e}"))?;
        let context = c2pa::Context::new()
            .with_settings(settings)
            .map_err(|e| anyhow::anyhow!("context: {e}"))?;
        c2pa::Reader::from_context(context).with_file(path)
    };
    match result {
        Ok(r) => Ok(Some(r)),
        Err(e) => {
            // If c2pa-rs can't parse the file, treat it as "no C2PA found"
            // rather than a fatal error. The prover has its own extraction.
            eprintln!("c2pa-rs could not read {path}: {e}");
            Ok(None)
        }
    }
}

/// Check whether signingCredential.untrusted is absent from validation statuses.
fn is_trusted(reader: &c2pa::Reader) -> bool {
    match reader.validation_status() {
        Some(statuses) => !statuses.iter().any(|s| s.code() == "signingCredential.untrusted"),
        None => true,
    }
}

/// Determine trust level by trying official list, then curated.
fn resolve_trust(
    path: &str,
    official_pem: &str,
    curated_pem: &str,
) -> Result<Option<(c2pa::Reader, String)>> {
    // 1. Try official trust list
    if !official_pem.is_empty() {
        match try_read(path, official_pem)? {
            None => return Ok(None),
            Some(r) if is_trusted(&r) => return Ok(Some((r, "official".into()))),
            Some(_) => {} // not trusted by official, fall through
        }
    }
    // 2. Try curated trust list
    if !curated_pem.is_empty() {
        match try_read(path, curated_pem)? {
            None => return Ok(None),
            Some(r) if is_trusted(&r) => return Ok(Some((r, "curated".into()))),
            Some(r) => return Ok(Some((r, "untrusted".into()))),
        }
    }
    // 3. No trust lists — still read the file
    match try_read(path, "")? {
        None => Ok(None),
        Some(r) => Ok(Some((r, "untrusted".into()))),
    }
}

/// Extract CN from an X.509 issuer DN string like "CN=Foo, O=Bar".
fn extract_cn(issuer: &str) -> Option<String> {
    issuer
        .split(',')
        .map(|s| s.trim())
        .find(|s| s.starts_with("CN="))
        .map(|s| s[3..].to_string())
}

#[derive(Default)]
struct Props {
    title: Option<String>,
    format: Option<String>,
    digital_source_type: Option<String>,
    claim_generator: Option<String>,
    software_agent: Option<String>,
    issuer: Option<String>,
    common_name: Option<String>,
    signing_time: Option<String>,
    sig_algorithm: Option<String>,
    actions: Option<Value>,
    ingredients: Option<Value>,
}

/// Pull flat provenance properties from the manifest store JSON.
fn extract_props(json: &Value) -> Props {
    let active_id = json
        .get("active_manifest")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let manifest = match json.get("manifests").and_then(|m| m.get(active_id)) {
        Some(m) => m,
        None => return Props::default(),
    };

    let title = manifest
        .get("title")
        .and_then(|v| v.as_str())
        .map(String::from);
    let format = manifest
        .get("format")
        .and_then(|v| v.as_str())
        .map(String::from);

    let claim_generator = manifest
        .get("claim_generator")
        .and_then(|v| v.as_str())
        .map(String::from);

    // Signature info
    let sig = manifest.get("signature_info");
    let issuer = sig
        .and_then(|s| s.get("issuer"))
        .and_then(|v| v.as_str())
        .map(String::from);
    let common_name = sig
        .and_then(|s| s.get("common_name"))
        .and_then(|v| v.as_str())
        .map(String::from)
        .or_else(|| issuer.as_deref().and_then(extract_cn));
    let signing_time = sig
        .and_then(|s| s.get("time"))
        .and_then(|v| v.as_str())
        .map(String::from);
    let sig_algorithm = sig
        .and_then(|s| s.get("alg"))
        .and_then(|v| v.as_str())
        .map(String::from);

    // Assertions
    let assertions = manifest.get("assertions").and_then(|v| v.as_array());
    let mut digital_source_type = None;
    let mut software_agent = None;
    let mut actions = None;

    if let Some(arr) = assertions {
        for a in arr {
            let label = a.get("label").and_then(|v| v.as_str()).unwrap_or("");
            let data = a.get("data");

            if label == "stds.schema-org.CreativeWork" {
                if let Some(d) = data {
                    digital_source_type = d
                        .get("digitalSourceType")
                        .and_then(|v| v.as_str())
                        .map(String::from);
                }
            } else if label.starts_with("c2pa.actions") {
                if let Some(d) = data {
                    actions = d.get("actions").cloned();
                    // Scan actions for softwareAgent and digitalSourceType
                    if let Some(action_arr) = d.get("actions").and_then(|a| a.as_array()) {
                        for act in action_arr {
                            if software_agent.is_none() {
                                software_agent = act.get("softwareAgent").and_then(|v| {
                                    v.as_str()
                                        .map(String::from)
                                        .or_else(|| v.get("name").and_then(|n| n.as_str()).map(String::from))
                                });
                            }
                            if digital_source_type.is_none() {
                                digital_source_type = act
                                    .get("digitalSourceType")
                                    .and_then(|v| v.as_str())
                                    .map(String::from);
                            }
                            // Check vendor-specific parameters
                            if let Some(params) = act.get("parameters") {
                                if digital_source_type.is_none() {
                                    digital_source_type = params
                                        .get("com.adobe.digitalSourceType")
                                        .and_then(|v| v.as_str())
                                        .map(String::from);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let ingredients = manifest.get("ingredients").cloned();

    // Fallback: search ingredient manifests for c2pa.created action data
    if digital_source_type.is_none() || software_agent.is_none() {
        if let Some(manifests) = json.get("manifests").and_then(|v| v.as_object()) {
            for (_, m) in manifests {
                if let Some(asserts) = m.get("assertions").and_then(|v| v.as_array()) {
                    for a in asserts {
                        let label = a.get("label").and_then(|v| v.as_str()).unwrap_or("");
                        if !label.starts_with("c2pa.actions") {
                            continue;
                        }
                        if let Some(action_arr) = a.get("data").and_then(|d| d.get("actions")).and_then(|a| a.as_array()) {
                            for act in action_arr {
                                if act.get("action").and_then(|v| v.as_str()) != Some("c2pa.created") {
                                    continue;
                                }
                                if digital_source_type.is_none() {
                                    digital_source_type = act
                                        .get("digitalSourceType")
                                        .and_then(|v| v.as_str())
                                        .map(String::from);
                                }
                                if software_agent.is_none() {
                                    software_agent = act.get("softwareAgent").and_then(|v| {
                                        v.as_str()
                                            .map(String::from)
                                            .or_else(|| v.get("name").and_then(|n| n.as_str()).map(String::from))
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Props {
        title,
        format,
        digital_source_type,
        claim_generator,
        software_agent,
        issuer,
        common_name,
        signing_time,
        sig_algorithm,
        actions,
        ingredients,
    }
}
