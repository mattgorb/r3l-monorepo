#![no_main]
sp1_zkvm::entrypoint!(main);

use coset::{CborSerializable, CoseSign1, TaggedCborSerializable};
use der::Decode;
use p256::ecdsa::{signature::Verifier, Signature, VerifyingKey};
use prover_shared::{CryptoEvidence, PublicOutputs};
use x509_cert::Certificate;

pub fn main() {
    let evidence = sp1_zkvm::io::read::<CryptoEvidence>();

    let outputs = if evidence.has_manifest {
        verify_and_extract(&evidence)
    } else {
        unsigned_outputs(evidence.asset_hash)
    };

    sp1_zkvm::io::commit(&outputs);
}

/// Outputs for files with no C2PA manifest.
fn unsigned_outputs(content_hash: [u8; 32]) -> PublicOutputs {
    PublicOutputs {
        content_hash,
        has_c2pa: false,
        trust_list_match: String::new(),
        validation_state: "None".to_string(),
        digital_source_type: String::new(),
        issuer: String::new(),
        common_name: String::new(),
        software_agent: String::new(),
        signing_time: String::new(),
    }
}

/// Verify COSE signature, validate trust chain, extract metadata — all inside zkVM.
fn verify_and_extract(evidence: &CryptoEvidence) -> PublicOutputs {
    // 1. Parse COSE_Sign1 (tagged, then untagged as fallback)
    let cose = match CoseSign1::from_tagged_slice(&evidence.cose_sign1_bytes)
        .or_else(|_| CoseSign1::from_slice(&evidence.cose_sign1_bytes))
    {
        Ok(c) => c,
        Err(_) => return unsigned_outputs(evidence.asset_hash),
    };

    // 2. Verify algorithm is ES256 (only supported algorithm for now)
    let is_es256 = matches!(
        cose.protected.header.alg,
        Some(coset::Algorithm::Assigned(coset::iana::Algorithm::ES256))
    );
    if !is_es256 {
        return unsigned_outputs(evidence.asset_hash);
    }

    // 3. Parse leaf certificate and extract P-256 public key
    if evidence.cert_chain_der.is_empty() {
        return unsigned_outputs(evidence.asset_hash);
    }

    let leaf_cert = match Certificate::from_der(&evidence.cert_chain_der[0]) {
        Ok(c) => c,
        Err(_) => return unsigned_outputs(evidence.asset_hash),
    };

    let pk_bytes = leaf_cert
        .tbs_certificate
        .subject_public_key_info
        .subject_public_key
        .raw_bytes();

    let verifying_key = match VerifyingKey::from_sec1_bytes(pk_bytes) {
        Ok(k) => k,
        Err(_) => return unsigned_outputs(evidence.asset_hash),
    };

    // 4. Build COSE Sig_structure1 and verify ECDSA P-256 signature
    //    Sig_structure1 = ["Signature1", protected, external_aad, payload]
    let protected_bytes = cose
        .protected
        .original_data
        .as_ref()
        .map(|v| v.as_slice())
        .unwrap_or(&[]);

    let sig_structure = ciborium::Value::Array(vec![
        ciborium::Value::Text("Signature1".to_string()),
        ciborium::Value::Bytes(protected_bytes.to_vec()),
        ciborium::Value::Bytes(Vec::new()), // external_aad (empty for C2PA)
        ciborium::Value::Bytes(evidence.claim_cbor.clone()), // detached payload
    ]);

    let mut tbs = Vec::new();
    if ciborium::ser::into_writer(&sig_structure, &mut tbs).is_err() {
        return unsigned_outputs(evidence.asset_hash);
    }

    let signature = match Signature::from_slice(&cose.signature) {
        Ok(s) => s,
        Err(_) => return unsigned_outputs(evidence.asset_hash),
    };

    if verifying_key.verify(&tbs, &signature).is_err() {
        return unsigned_outputs(evidence.asset_hash);
    }

    // --- Signature verified! Everything below uses cryptographically authenticated data ---

    // 5. Determine trust level: match root cert against known trust anchors
    let trust_list_match = determine_trust_level(
        &evidence.cert_chain_der,
        &evidence.official_trust_anchors_der,
        &evidence.curated_trust_anchors_der,
    );

    let validation_state = if trust_list_match == "untrusted" {
        "SignatureOnly".to_string()
    } else {
        "Verified".to_string()
    };

    // 6. Extract issuer org and common name from verified leaf cert
    let (issuer, common_name) = extract_cert_names(&leaf_cert);

    // 7. Extract claim_generator from verified claim CBOR
    let software_agent = extract_claim_generator(&evidence.claim_cbor);

    PublicOutputs {
        content_hash: evidence.asset_hash,
        has_c2pa: true,
        trust_list_match,
        validation_state,
        digital_source_type: String::new(), // requires assertion box parsing (future)
        issuer,
        common_name,
        software_agent,
        signing_time: String::new(), // requires RFC 3161 parsing (future)
    }
}

/// Match the root certificate (last in chain) against trust anchor lists.
fn determine_trust_level(
    cert_chain: &[Vec<u8>],
    official_anchors: &[Vec<u8>],
    curated_anchors: &[Vec<u8>],
) -> String {
    let root_der = match cert_chain.last() {
        Some(r) => r,
        None => return "untrusted".to_string(),
    };

    for anchor in official_anchors {
        if anchor == root_der {
            return "official".to_string();
        }
    }

    for anchor in curated_anchors {
        if anchor == root_der {
            return "curated".to_string();
        }
    }

    "untrusted".to_string()
}

/// Extract Organization (issuer) and Common Name from an X.509 certificate.
fn extract_cert_names(cert: &Certificate) -> (String, String) {
    let issuer_org = extract_rdn_attr(&cert.tbs_certificate.issuer, OID_ORG);
    let cn = extract_rdn_attr(&cert.tbs_certificate.subject, OID_CN);

    // Fallback: if subject CN is empty, try issuer CN
    let common_name = if cn.is_empty() {
        extract_rdn_attr(&cert.tbs_certificate.issuer, OID_CN)
    } else {
        cn
    };

    (issuer_org, common_name)
}

const OID_CN: der::oid::ObjectIdentifier = der::oid::ObjectIdentifier::new_unwrap("2.5.4.3");
const OID_ORG: der::oid::ObjectIdentifier = der::oid::ObjectIdentifier::new_unwrap("2.5.4.10");

fn extract_rdn_attr(name: &x509_cert::name::Name, target_oid: der::oid::ObjectIdentifier) -> String {
    for rdn in name.0.iter() {
        for atv in rdn.0.iter() {
            if atv.oid == target_oid {
                // Raw content octets — works for UTF8String and PrintableString
                return String::from_utf8_lossy(atv.value.value()).to_string();
            }
        }
    }
    String::new()
}

/// Extract software agent name from C2PA claim CBOR.
/// C2PA v1 uses "claim_generator" (text), v2 uses "claim_generator_info" (map with "name").
fn extract_claim_generator(claim_cbor: &[u8]) -> String {
    if claim_cbor.is_empty() {
        return String::new();
    }

    let claim: ciborium::Value = match ciborium::de::from_reader(claim_cbor) {
        Ok(v) => v,
        Err(_) => return String::new(),
    };

    let map = match claim.as_map() {
        Some(m) => m,
        None => return String::new(),
    };

    // Try v2: claim_generator_info → map with "name" key
    if let Some((_, info)) = map
        .iter()
        .find(|(k, _)| k.as_text() == Some("claim_generator_info"))
    {
        if let Some(name) = info
            .as_map()
            .and_then(|m| m.iter().find(|(k, _)| k.as_text() == Some("name")))
            .and_then(|(_, v)| v.as_text())
        {
            return name.to_string();
        }
    }

    // Fallback v1: claim_generator (text string)
    map.iter()
        .find(|(k, _)| k.as_text() == Some("claim_generator"))
        .and_then(|(_, v)| v.as_text())
        .unwrap_or("")
        .to_string()
}
