use anchor_lang::prelude::*;

mod constants;
mod errors;
mod state;

use constants::{ATTESTATION_SEED, SP1_VKEY_HASH};
use errors::ProvenanceError;
use state::Attestation;

declare_id!("HahVgC9uo73aLw1ouBEvgMT7KmGTS6rovfbKP9zuCtjc");

#[program]
pub mod provenance_attestation {
    use super::*;

    /// Submit a ZK proof to store a C2PA provenance attestation.
    /// Anyone can call this — the Groth16 proof is the authorization.
    pub fn submit_proof(
        ctx: Context<SubmitProof>,
        proof: Vec<u8>,
        public_inputs: Vec<u8>,
        // Attestation fields (decoded from public outputs by the client)
        content_hash: [u8; 32],
        has_c2pa: bool,
        trust_list_match: String,
        validation_state: String,
        digital_source_type: String,
        issuer: String,
        common_name: String,
        software_agent: String,
        signing_time: String,
    ) -> Result<()> {
        // Verify the Groth16 proof on-chain
        // TODO: uncomment when sp1-solana is integrated
        // sp1_solana::verify_proof(
        //     &proof,
        //     &public_inputs,
        //     SP1_VKEY_HASH,
        //     sp1_solana::GROTH16_VK_5_0_0_BYTES,
        // ).map_err(|_| ProvenanceError::ProofVerificationFailed)?;

        // TODO: verify that content_hash and attestation fields match the
        // public outputs in the proof. For now we trust the client's decoding.

        // Validate string lengths
        require!(trust_list_match.len() <= Attestation::MAX_STRING_LEN, ProvenanceError::StringTooLong);
        require!(validation_state.len() <= Attestation::MAX_STRING_LEN, ProvenanceError::StringTooLong);
        require!(digital_source_type.len() <= Attestation::MAX_STRING_LEN, ProvenanceError::StringTooLong);
        require!(issuer.len() <= Attestation::MAX_STRING_LEN, ProvenanceError::StringTooLong);
        require!(common_name.len() <= Attestation::MAX_STRING_LEN, ProvenanceError::StringTooLong);
        require!(software_agent.len() <= Attestation::MAX_STRING_LEN, ProvenanceError::StringTooLong);
        require!(signing_time.len() <= Attestation::MAX_STRING_LEN, ProvenanceError::StringTooLong);

        // Store attestation
        let attestation = &mut ctx.accounts.attestation;
        attestation.content_hash = content_hash;
        attestation.has_c2pa = has_c2pa;
        attestation.trust_list_match = trust_list_match;
        attestation.validation_state = validation_state;
        attestation.digital_source_type = digital_source_type;
        attestation.issuer = issuer;
        attestation.common_name = common_name;
        attestation.software_agent = software_agent;
        attestation.signing_time = signing_time;
        attestation.submitted_by = ctx.accounts.submitter.key();
        attestation.timestamp = Clock::get()?.unix_timestamp;
        attestation.bump = ctx.bumps.attestation;

        msg!("Attestation stored for content_hash: {:?}", hex::encode(content_hash));

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(
    proof: Vec<u8>,
    public_inputs: Vec<u8>,
    content_hash: [u8; 32],
)]
pub struct SubmitProof<'info> {
    #[account(
        init,
        payer = submitter,
        space = Attestation::SPACE,
        seeds = [ATTESTATION_SEED, content_hash.as_ref()],
        bump,
    )]
    pub attestation: Account<'info, Attestation>,
    #[account(mut)]
    pub submitter: Signer<'info>,
    pub system_program: Program<'info, System>,
}
