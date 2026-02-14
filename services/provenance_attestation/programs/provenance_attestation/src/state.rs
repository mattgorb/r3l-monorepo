use anchor_lang::prelude::*;

/// On-chain attestation record, one per file.
/// PDA seeded by [b"attestation", content_hash].
#[account]
pub struct Attestation {
    /// SHA-256 of the original file bytes
    pub content_hash: [u8; 32],
    /// Whether the file had valid C2PA metadata
    pub has_c2pa: bool,
    /// "official", "curated", or "untrusted"
    pub trust_list_match: String,
    /// "Trusted", "Valid", or "Invalid"
    pub validation_state: String,
    /// IPTC digital source type URI
    pub digital_source_type: String,
    /// Certificate issuer organization
    pub issuer: String,
    /// Certificate common name
    pub common_name: String,
    /// Content creation tool
    pub software_agent: String,
    /// ISO timestamp of signature
    pub signing_time: String,
    /// SHA-256 fingerprint of the leaf signing certificate (hex)
    pub cert_fingerprint: String,
    /// Who submitted the transaction
    pub submitted_by: Pubkey,
    /// Solana clock timestamp
    pub timestamp: i64,
    /// PDA bump seed
    pub bump: u8,
}

impl Attestation {
    /// Max size for each string field (bytes)
    pub const MAX_STRING_LEN: usize = 128;

    /// Space needed for the account:
    /// 8 (discriminator) + 32 (content_hash) + 1 (has_c2pa) +
    /// 8 * (4 + MAX_STRING_LEN) (strings with length prefix) +
    /// 32 (submitted_by) + 8 (timestamp) + 1 (bump)
    pub const SPACE: usize = 8 + 32 + 1 + 8 * (4 + Self::MAX_STRING_LEN) + 32 + 8 + 1;
}
