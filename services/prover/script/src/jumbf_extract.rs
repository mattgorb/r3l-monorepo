//! Extract C2PA cryptographic evidence from PNG files.
//!
//! Pipeline: PNG → caBX chunk → JUMBF box tree → claim CBOR + COSE_Sign1,
//! then extract certificate chain from COSE unprotected header.

use anyhow::{anyhow, Context, Result};
use prover_shared::CryptoEvidence;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

/// Read a media file and trust directories, return CryptoEvidence for the zkVM guest.
pub fn extract_crypto_evidence(media_path: &str, trust_dir: &str) -> Result<CryptoEvidence> {
    let file_bytes =
        fs::read(media_path).with_context(|| format!("reading media file: {media_path}"))?;

    let asset_hash: [u8; 32] = Sha256::digest(&file_bytes).into();

    // Extract C2PA JUMBF data from PNG
    let jumbf_data = extract_c2pa_from_png(&file_bytes);

    let (has_manifest, cose_sign1_bytes, cert_chain_der, claim_cbor) = match jumbf_data {
        Some(jumbf) => {
            eprintln!(
                "Found C2PA JUMBF data: {} bytes",
                jumbf.len()
            );
            match extract_claim_and_signature(&jumbf) {
                Some((claim, sig)) => {
                    eprintln!(
                        "Extracted claim ({} bytes) and signature ({} bytes)",
                        claim.len(),
                        sig.len()
                    );
                    let certs = extract_cert_chain_from_cose(&sig).unwrap_or_else(|e| {
                        eprintln!("Warning: failed to extract cert chain: {e}");
                        Vec::new()
                    });
                    eprintln!("Extracted {} certificate(s) from COSE x5chain", certs.len());
                    (true, sig, certs, claim)
                }
                None => {
                    eprintln!("Warning: JUMBF found but could not extract claim/signature boxes");
                    (false, Vec::new(), Vec::new(), Vec::new())
                }
            }
        }
        None => {
            eprintln!("No C2PA JUMBF data found in PNG");
            (false, Vec::new(), Vec::new(), Vec::new())
        }
    };

    // Load trust anchors from PEM directories
    let trust_path = Path::new(trust_dir);
    let official = load_trust_anchors_der(&trust_path.join("official"))?;
    let curated = load_trust_anchors_der(&trust_path.join("curated"))?;
    eprintln!(
        "Loaded {} official + {} curated trust anchors",
        official.len(),
        curated.len()
    );

    Ok(CryptoEvidence {
        asset_hash,
        has_manifest,
        cose_sign1_bytes,
        cert_chain_der,
        claim_cbor,
        official_trust_anchors_der: official,
        curated_trust_anchors_der: curated,
    })
}

// ---------------------------------------------------------------------------
// PNG chunk parsing
// ---------------------------------------------------------------------------

/// Extract C2PA JUMBF data from PNG caBX chunk(s).
fn extract_c2pa_from_png(data: &[u8]) -> Option<Vec<u8>> {
    const PNG_SIG: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    if !data.starts_with(PNG_SIG) {
        return None;
    }

    let mut jumbf = Vec::new();
    let mut pos = 8; // skip PNG signature

    while pos + 12 <= data.len() {
        let length = u32::from_be_bytes(data[pos..pos + 4].try_into().ok()?) as usize;
        let chunk_type = &data[pos + 4..pos + 8];
        let data_start = pos + 8;
        let data_end = data_start + length;

        if data_end + 4 > data.len() {
            break;
        }

        if chunk_type == b"caBX" {
            jumbf.extend_from_slice(&data[data_start..data_end]);
        }

        pos = data_end + 4; // skip CRC
    }

    if jumbf.is_empty() {
        None
    } else {
        Some(jumbf)
    }
}

// ---------------------------------------------------------------------------
// JUMBF / ISO BMFF box parsing
// ---------------------------------------------------------------------------

struct BmffBox<'a> {
    box_type: [u8; 4],
    data: &'a [u8], // content after the 8-byte header
}

/// Parse consecutive ISO BMFF boxes from a byte slice.
fn parse_boxes(data: &[u8]) -> Vec<BmffBox<'_>> {
    let mut result = Vec::new();
    let mut pos = 0;

    while pos + 8 <= data.len() {
        let size =
            u32::from_be_bytes(data[pos..pos + 4].try_into().unwrap_or([0; 4])) as usize;

        if size < 8 || pos + size > data.len() {
            break;
        }

        let box_type: [u8; 4] = data[pos + 4..pos + 8].try_into().unwrap_or([0; 4]);
        let content = &data[pos + 8..pos + size];

        result.push(BmffBox {
            box_type,
            data: content,
        });
        pos += size;
    }

    result
}

/// Extract the label from a JUMD (JUMBF Description) box's content.
/// Layout: [UUID:16][toggles:1][label?][id?][hash?]
fn parse_jumd_label(data: &[u8]) -> Option<String> {
    if data.len() < 17 {
        return None;
    }

    let toggles = data[16];
    let has_label = toggles & 0x02 != 0;

    if !has_label {
        return None;
    }

    let label_start = 17;
    let null_pos = data[label_start..].iter().position(|&b| b == 0)?;
    std::str::from_utf8(&data[label_start..label_start + null_pos])
        .ok()
        .map(String::from)
}

/// Walk the JUMBF box tree to find claim CBOR and COSE_Sign1 signature
/// from the active (last) manifest.
fn extract_claim_and_signature(jumbf: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    let top_boxes = parse_boxes(jumbf);

    // Top-level should be a single jumb box (C2PA manifest store)
    let store = top_boxes.iter().find(|b| &b.box_type == b"jumb")?;
    let store_children = parse_boxes(store.data);

    // Active manifest = last jumb child in the store (per C2PA spec)
    let last_manifest = store_children
        .iter()
        .rev()
        .find(|b| &b.box_type == b"jumb")?;

    let manifest_children = parse_boxes(last_manifest.data);

    let mut claim_cbor = None;
    let mut cose_sign1 = None;

    for child in &manifest_children {
        if &child.box_type != b"jumb" {
            continue;
        }

        let inner = parse_boxes(child.data);
        let label = inner
            .first()
            .filter(|b| &b.box_type == b"jumd")
            .and_then(|b| parse_jumd_label(b.data));

        match label.as_deref() {
            Some(l) if l.starts_with("c2pa.claim") => {
                // Content is in the box after the jumd (raw CBOR)
                // Handles both "c2pa.claim" and "c2pa.claim.v2"
                if let Some(content_box) = inner.get(1) {
                    claim_cbor = Some(content_box.data.to_vec());
                }
            }
            Some(l) if l.starts_with("c2pa.signature") => {
                // Content may be in a bfdb (embedded file) box — skip its header
                if let Some(content_box) = inner.get(1) {
                    let raw = extract_embedded_content(content_box);
                    cose_sign1 = Some(raw.to_vec());
                }
            }
            _ => {}
        }
    }

    match (claim_cbor, cose_sign1) {
        (Some(claim), Some(sig)) => Some((claim, sig)),
        _ => None,
    }
}

/// For a bfdb (embedded file content) box, skip the toggle byte and
/// optional media-type/filename strings to get to the raw content.
/// For any other box type, return the data as-is.
fn extract_embedded_content<'a>(content_box: &BmffBox<'a>) -> &'a [u8] {
    if &content_box.box_type == b"bfdb" {
        skip_bfdb_header(content_box.data)
    } else {
        content_box.data
    }
}

fn skip_bfdb_header(data: &[u8]) -> &[u8] {
    if data.is_empty() {
        return data;
    }
    let toggle = data[0];
    let mut pos = 1;

    // Bit 0: media type present (null-terminated string)
    if toggle & 0x01 != 0 {
        if let Some(null_pos) = data[pos..].iter().position(|&b| b == 0) {
            pos += null_pos + 1;
        }
    }
    // Bit 1: file name present (null-terminated string)
    if toggle & 0x02 != 0 {
        if let Some(null_pos) = data[pos..].iter().position(|&b| b == 0) {
            pos += null_pos + 1;
        }
    }

    &data[pos..]
}

// ---------------------------------------------------------------------------
// COSE certificate extraction
// ---------------------------------------------------------------------------

/// Extract DER-encoded certificate chain from COSE_Sign1 x5chain header.
fn extract_cert_chain_from_cose(cose_bytes: &[u8]) -> Result<Vec<Vec<u8>>> {
    use coset::{CborSerializable, CoseSign1, Label, TaggedCborSerializable};

    // C2PA uses COSE_Sign1_Tagged (CBOR tag 18). Try tagged first, then untagged.
    let cose = CoseSign1::from_tagged_slice(cose_bytes)
        .or_else(|_| CoseSign1::from_slice(cose_bytes))
        .map_err(|e| anyhow!("parsing COSE_Sign1: {e}"))?;

    // x5chain = COSE header parameter label 33
    let x5chain_label = Label::Int(33);

    // Check unprotected header first (standard location), then protected
    let value = cose
        .unprotected
        .rest
        .iter()
        .find(|(k, _)| k == &x5chain_label)
        .map(|(_, v)| v)
        .or_else(|| {
            cose.protected
                .header
                .rest
                .iter()
                .find(|(k, _)| k == &x5chain_label)
                .map(|(_, v)| v)
        });

    let Some(val) = value else {
        return Ok(Vec::new());
    };

    // x5chain can be a single bstr (one cert) or array of bstr (chain, leaf first)
    match val {
        ciborium::Value::Bytes(der) => Ok(vec![der.clone()]),
        ciborium::Value::Array(arr) => Ok(arr
            .iter()
            .filter_map(|v| {
                if let ciborium::Value::Bytes(der) = v {
                    Some(der.clone())
                } else {
                    None
                }
            })
            .collect()),
        _ => Ok(Vec::new()),
    }
}

// ---------------------------------------------------------------------------
// Trust anchor loading
// ---------------------------------------------------------------------------

/// Load all PEM certificates from a directory, return DER-encoded bytes.
fn load_trust_anchors_der(dir: &Path) -> Result<Vec<Vec<u8>>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut anchors = Vec::new();
    let mut entries: Vec<_> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "pem"))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let pem_str = fs::read_to_string(entry.path())
            .with_context(|| format!("reading PEM: {}", entry.path().display()))?;
        let pems = pem::parse_many(&pem_str)
            .map_err(|e| anyhow!("parsing PEM {}: {e}", entry.path().display()))?;
        for p in pems {
            anchors.push(p.into_contents());
        }
    }

    Ok(anchors)
}
