use anchor_lang::prelude::*;

#[error_code]
pub enum ProvenanceError {
    #[msg("ZK proof verification failed")]
    ProofVerificationFailed,
    #[msg("Invalid public outputs in proof")]
    InvalidPublicOutputs,
    #[msg("String field exceeds maximum length")]
    StringTooLong,
    #[msg("Content hash does not match proof public outputs")]
    ContentHashMismatch,
}
