//! Extract C2PA cryptographic evidence from media files (PNG, JPEG, MP4).
//!
//! Supported formats:
//!   PNG  — caBX chunk(s) contain raw JUMBF data
//!   JPEG — APP11 (0xFFEB) marker segments per ISO 19566-5 (JUMBF-in-JPEG)
//!   MP4  — top-level BMFF `uuid` box with C2PA UUID
//!
//! Pipeline: media → JUMBF → box tree → claim CBOR + COSE_Sign1 +
//! assertion boxes, then extract certificate chain from COSE unprotected header.

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

    // Detect file type and extract C2PA JUMBF data
    let (format_name, jumbf_data) = if file_bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        ("PNG", extract_c2pa_from_png(&file_bytes))
    } else if file_bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        ("JPEG", extract_c2pa_from_jpeg(&file_bytes))
    } else if is_bmff(&file_bytes) {
        ("MP4/BMFF", extract_c2pa_from_bmff(&file_bytes))
    } else {
        ("unknown", None)
    };
    eprintln!("Detected format: {format_name}");

    let (has_manifest, cose_sign1_bytes, cert_chain_der, claim_cbor, assertion_boxes) =
        match jumbf_data {
            Some(jumbf) => {
                eprintln!("Found C2PA JUMBF data: {} bytes", jumbf.len());
                match extract_manifest_parts(&jumbf) {
                    Some((claim, sig, assertions)) => {
                        eprintln!(
                            "Extracted claim ({} bytes), signature ({} bytes), {} assertion(s)",
                            claim.len(),
                            sig.len(),
                            assertions.len()
                        );
                        for (label, data) in &assertions {
                            eprintln!("  assertion: {} ({} bytes)", label, data.len());
                        }
                        let certs = extract_cert_chain_from_cose(&sig).unwrap_or_else(|e| {
                            eprintln!("Warning: failed to extract cert chain: {e}");
                            Vec::new()
                        });
                        eprintln!(
                            "Extracted {} certificate(s) from COSE x5chain",
                            certs.len()
                        );
                        (true, sig, certs, claim, assertions)
                    }
                    None => {
                        eprintln!(
                            "Warning: JUMBF found but could not extract claim/signature boxes"
                        );
                        (false, Vec::new(), Vec::new(), Vec::new(), Vec::new())
                    }
                }
            }
            None => {
                eprintln!("No C2PA JUMBF data found in {format_name} file");
                (false, Vec::new(), Vec::new(), Vec::new(), Vec::new())
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
        assertion_boxes,
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
// JPEG APP11 parsing (ISO 19566-5: JUMBF in JPEG)
// ---------------------------------------------------------------------------

/// Extract C2PA JUMBF data from JPEG APP11 marker segments.
///
/// Per ISO 19566-5, each APP11 segment carries a JUMBF fragment:
///   marker (0xFFEB) | Lp (2 bytes) | CI="JP" (2) | En (2) | Z (4) | data...
///
/// Fragments with the same En (box instance) are sorted by Z (packet
/// sequence number, 1-based) and concatenated to form the complete JUMBF.
fn extract_c2pa_from_jpeg(data: &[u8]) -> Option<Vec<u8>> {
    if !data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return None;
    }

    // Collect (En, Z, payload) tuples from all APP11 segments
    let mut fragments: Vec<(u16, u32, Vec<u8>)> = Vec::new();
    let mut pos = 2; // skip SOI (0xFFD8)

    while pos + 1 < data.len() {
        // Find next marker
        if data[pos] != 0xFF {
            pos += 1;
            continue;
        }
        let marker = data[pos + 1];
        pos += 2;

        // Skip padding 0xFF bytes
        if marker == 0xFF || marker == 0x00 {
            continue;
        }

        // SOS (0xDA) — start of scan, everything after is entropy-coded data
        if marker == 0xDA {
            break;
        }

        // Standalone markers (RST, SOI, EOI) have no length
        if marker == 0xD8 || marker == 0xD9 || (0xD0..=0xD7).contains(&marker) {
            continue;
        }

        // All other markers have a 2-byte length field
        if pos + 2 > data.len() {
            break;
        }
        let seg_len = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
        if seg_len < 2 || pos + seg_len > data.len() {
            break;
        }

        // APP11 = 0xEB
        if marker == 0xEB && seg_len >= 10 {
            let seg_data = &data[pos..pos + seg_len];
            // seg_data[0..2] = Lp (length, already consumed)
            // seg_data[2..4] = CI, should be "JP"
            if &seg_data[2..4] == b"JP" {
                let en = u16::from_be_bytes([seg_data[4], seg_data[5]]);
                let z = u32::from_be_bytes([seg_data[6], seg_data[7], seg_data[8], seg_data[9]]);
                let payload = seg_data[10..].to_vec();
                fragments.push((en, z, payload));
            }
        }

        pos += seg_len;
    }

    if fragments.is_empty() {
        return None;
    }

    // Sort by En then Z, concatenate fragments for each box instance
    fragments.sort_by_key(|&(en, z, _)| (en, z));

    // Use the first En value (typically there's only one C2PA JUMBF instance)
    let target_en = fragments[0].0;
    let mut jumbf = Vec::new();
    for (en, _z, payload) in &fragments {
        if *en == target_en {
            jumbf.extend_from_slice(payload);
        }
    }

    if jumbf.is_empty() {
        None
    } else {
        Some(jumbf)
    }
}

// ---------------------------------------------------------------------------
// MP4/BMFF C2PA extraction
// ---------------------------------------------------------------------------

/// C2PA JUMBF UUID for BMFF containers: d8fec3d6-1b0e-483c-9297-5828877ec481
const C2PA_UUID: [u8; 16] = [
    0xd8, 0xfe, 0xc3, 0xd6, 0x1b, 0x0e, 0x48, 0x3c, 0x92, 0x97, 0x58, 0x28, 0x87, 0x7e, 0xc4,
    0x81,
];

/// Check if a file is BMFF-based (MP4, MOV, HEIF, etc.) by looking for `ftyp` box.
fn is_bmff(data: &[u8]) -> bool {
    // BMFF files start with a box whose type is `ftyp` at offset 4
    data.len() >= 8 && &data[4..8] == b"ftyp"
}

/// Extract C2PA JUMBF data from a BMFF container (MP4, MOV, HEIF, etc.).
///
/// Scans top-level boxes for a `uuid` box with the C2PA UUID.
/// The C2PA UUID box has internal structure (per c2pa-rs/C2PA spec):
///   [16 bytes: UUID][4 bytes: FullBox version+flags][null-terminated purpose string]
///   [8 bytes: aux uuid offset][JUMBF manifest data...]
fn extract_c2pa_from_bmff(data: &[u8]) -> Option<Vec<u8>> {
    let mut pos = 0;

    while pos + 8 <= data.len() {
        let size = u32::from_be_bytes(data[pos..pos + 4].try_into().ok()?) as u64;
        let box_type = &data[pos + 4..pos + 8];

        let (header_size, box_size) = if size == 1 {
            // Extended size: 64-bit size follows the box type
            if pos + 16 > data.len() {
                break;
            }
            let ext_size = u64::from_be_bytes(data[pos + 8..pos + 16].try_into().ok()?);
            (16u64, ext_size)
        } else if size == 0 {
            // Box extends to end of file
            (8u64, (data.len() - pos) as u64)
        } else {
            (8u64, size)
        };

        if box_size < header_size || pos as u64 + box_size > data.len() as u64 {
            break;
        }

        if box_type == b"uuid" {
            let content_start = pos + header_size as usize;
            let content_end = pos + box_size as usize;
            let content = &data[content_start..content_end];

            // uuid box content: 16-byte UUID + C2PA envelope
            if content.len() >= 16 && content[..16] == C2PA_UUID {
                let inner = &content[16..];
                // Skip FullBox header (4 bytes: version + flags)
                if inner.len() < 4 {
                    continue;
                }
                let mut cursor = 4usize;

                // Read null-terminated purpose string
                let null_pos = inner[cursor..].iter().position(|&b| b == 0)?;
                let purpose = std::str::from_utf8(&inner[cursor..cursor + null_pos]).ok()?;
                cursor += null_pos + 1; // skip string + null

                if purpose != "manifest" && purpose != "original" {
                    continue;
                }

                // Skip 8-byte aux uuid offset
                if cursor + 8 > inner.len() {
                    continue;
                }
                cursor += 8;

                return Some(inner[cursor..].to_vec());
            }
        }

        pos += box_size as usize;
    }

    None
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

/// Walk the JUMBF box tree to find claim CBOR, COSE_Sign1 signature,
/// and assertion boxes. Claim + signature come from the active (last) manifest.
/// Assertions are collected from ALL manifests so ingredient metadata is available.
/// Returns (claim_cbor, cose_sign1, assertion_boxes).
fn extract_manifest_parts(
    jumbf: &[u8],
) -> Option<(Vec<u8>, Vec<u8>, Vec<(String, Vec<u8>)>)> {
    let top_boxes = parse_boxes(jumbf);

    // Top-level should be a single jumb box (C2PA manifest store)
    let store = top_boxes.iter().find(|b| &b.box_type == b"jumb")?;
    let store_children = parse_boxes(store.data);

    let manifests: Vec<_> = store_children
        .iter()
        .filter(|b| &b.box_type == b"jumb")
        .collect();

    if manifests.is_empty() {
        return None;
    }

    // Active manifest = last jumb child in the store (per C2PA spec)
    let active = manifests.last().unwrap();
    let active_children = parse_boxes(active.data);

    let mut claim_cbor = None;
    let mut cose_sign1 = None;

    for child in &active_children {
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
                if let Some(content_box) = inner.get(1) {
                    claim_cbor = Some(content_box.data.to_vec());
                }
            }
            Some(l) if l.starts_with("c2pa.signature") => {
                if let Some(content_box) = inner.get(1) {
                    let raw = extract_embedded_content(content_box);
                    cose_sign1 = Some(raw.to_vec());
                }
            }
            _ => {}
        }
    }

    // Collect assertions from ALL manifests (active + ingredients)
    let mut assertions = Vec::new();
    for manifest in &manifests {
        let children = parse_boxes(manifest.data);
        for child in &children {
            if &child.box_type != b"jumb" {
                continue;
            }
            let inner = parse_boxes(child.data);
            let label = inner
                .first()
                .filter(|b| &b.box_type == b"jumd")
                .and_then(|b| parse_jumd_label(b.data));
            if let Some(l) = &label {
                if l == "c2pa.assertions" {
                    extract_assertions_from_store(&inner[1..], &mut assertions);
                }
            }
        }
    }

    match (claim_cbor, cose_sign1) {
        (Some(claim), Some(sig)) => Some((claim, sig, assertions)),
        _ => None,
    }
}

/// Parse individual assertion boxes from an assertion store superbox.
fn extract_assertions_from_store(
    children: &[BmffBox<'_>],
    out: &mut Vec<(String, Vec<u8>)>,
) {
    for child in children {
        if &child.box_type != b"jumb" {
            continue;
        }
        let inner = parse_boxes(child.data);
        let label = inner
            .first()
            .filter(|b| &b.box_type == b"jumd")
            .and_then(|b| parse_jumd_label(b.data));
        if let Some(al) = label {
            if let Some(content_box) = inner.get(1) {
                let raw = extract_embedded_content(content_box);
                out.push((al, raw.to_vec()));
            }
        }
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

    // x5chain = COSE header parameter label 33 (integer) or "x5chain" (text)
    let x5chain_int = Label::Int(33);
    let x5chain_text = Label::Text("x5chain".to_string());

    // Check unprotected header first (standard location), then protected
    let value = cose
        .unprotected
        .rest
        .iter()
        .find(|(k, _)| k == &x5chain_int || k == &x5chain_text)
        .map(|(_, v)| v)
        .or_else(|| {
            cose.protected
                .header
                .rest
                .iter()
                .find(|(k, _)| k == &x5chain_int || k == &x5chain_text)
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
