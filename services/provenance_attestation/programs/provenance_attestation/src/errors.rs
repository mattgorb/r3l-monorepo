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
    #[msg("Unauthorized: only the R3L authority can submit attestations")]
    Unauthorized,
    #[msg("Domain must not be empty")]
    DomainEmpty,
    #[msg("Invalid or missing Ed25519 signature verification instruction")]
    InvalidWalletSigVerify,
    #[msg("Wallet pubkey in Ed25519 instruction does not match wallet parameter")]
    WalletPubkeyMismatch,
}
