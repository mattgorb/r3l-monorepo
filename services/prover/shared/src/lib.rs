use serde::{Deserialize, Serialize};

/// Private inputs fed from host to guest.
/// The host extracts raw crypto evidence from the C2PA manifest;
/// the guest re-verifies the cryptographic primitives inside the zkVM.
#[derive(Serialize, Deserialize)]
pub struct CryptoEvidence {
    /// SHA-256 hash of the original asset (computed outside zkVM for efficiency)
    pub asset_hash: [u8; 32],
    /// Whether the file had a C2PA manifest
    pub has_manifest: bool,
    /// Raw COSE_Sign1_Tagged bytes (the entire COSE structure from the signature box)
    pub cose_sign1_bytes: Vec<u8>,
    /// X.509 certificate chain (DER-encoded, leaf first)
    pub cert_chain_der: Vec<Vec<u8>>,
    /// Raw CBOR claim payload bytes (from the c2pa.claim box, detached payload)
    pub claim_cbor: Vec<u8>,
    /// Official trust anchor certificates (DER-encoded)
    pub official_trust_anchors_der: Vec<Vec<u8>>,
    /// Curated trust anchor certificates (DER-encoded)
    pub curated_trust_anchors_der: Vec<Vec<u8>>,
}

/// Public outputs committed by the guest.
/// These become the attestation fields stored on-chain.
/// All fields are derived from cryptographically verified data inside the zkVM.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PublicOutputs {
    /// SHA-256 of the original file bytes
    pub content_hash: [u8; 32],
    /// Whether the file had valid C2PA metadata with a verified signature
    pub has_c2pa: bool,
    /// "official", "curated", or "untrusted"
    pub trust_list_match: String,
    /// "Verified" (sig + trust anchor valid), "SignatureOnly" (sig valid, no trust anchor), or "None"
    pub validation_state: String,
    /// IPTC digital source type URI (from claim, if available)
    pub digital_source_type: String,
    /// Certificate issuer organization (from verified leaf cert)
    pub issuer: String,
    /// Certificate common name (from verified leaf cert)
    pub common_name: String,
    /// Content creation tool (from claim_generator in verified claim)
    pub software_agent: String,
    /// ISO timestamp of signature (from COSE protected header, if present)
    pub signing_time: String,
}
