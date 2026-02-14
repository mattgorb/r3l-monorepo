use anchor_lang::prelude::*;

mod constants;
mod errors;
mod state;

use constants::{ATTESTATION_SEED, SP1_VKEY_HASH};
use errors::ProvenanceError;
use state::Attestation;

declare_id!("kK63ncGUJXCWjUqSYzcN92tW531rt4UtacJBmHWXJ16");

#[program]
pub mod provenance_attestation {
    use super::*;

    /// Submit a ZK proof to store a C2PA provenance attestation.
    /// Anyone can call this — the Groth16 proof is the authorization.
    ///
    /// The `public_inputs` bytes are bincode-serialized PublicOutputs from the SP1 guest.
    /// After proof verification, we parse them on-chain and store directly — no client-supplied
    /// attestation fields needed. The `content_hash` arg is only for PDA seed derivation
    /// and is verified against the parsed public outputs.
    pub fn submit_proof(
        ctx: Context<SubmitProof>,
        proof: Vec<u8>,
        public_inputs: Vec<u8>,
        content_hash: [u8; 32],
    ) -> Result<()> {
        // 1. Verify the Groth16 proof on-chain
        #[cfg(not(feature = "skip-verification"))]
        {
            sp1_solana::verify_proof(
                &proof,
                &public_inputs,
                SP1_VKEY_HASH,
                sp1_solana::GROTH16_VK_5_0_0_BYTES,
            )
            .map_err(|_| ProvenanceError::ProofVerificationFailed)?;
        }

        // Suppress unused variable warning when verification is skipped
        #[cfg(feature = "skip-verification")]
        let _ = &proof;

        // 2. Parse PublicOutputs from the cryptographically verified public_inputs
        let outputs = parse_public_outputs(&public_inputs)?;

        // 3. Verify content_hash matches what the proof committed to
        require!(
            outputs.content_hash == content_hash,
            ProvenanceError::ContentHashMismatch
        );

        // 4. Validate string lengths
        require!(
            outputs.trust_list_match.len() <= Attestation::MAX_STRING_LEN,
            ProvenanceError::StringTooLong
        );
        require!(
            outputs.validation_state.len() <= Attestation::MAX_STRING_LEN,
            ProvenanceError::StringTooLong
        );
        require!(
            outputs.digital_source_type.len() <= Attestation::MAX_STRING_LEN,
            ProvenanceError::StringTooLong
        );
        require!(
            outputs.issuer.len() <= Attestation::MAX_STRING_LEN,
            ProvenanceError::StringTooLong
        );
        require!(
            outputs.common_name.len() <= Attestation::MAX_STRING_LEN,
            ProvenanceError::StringTooLong
        );
        require!(
            outputs.software_agent.len() <= Attestation::MAX_STRING_LEN,
            ProvenanceError::StringTooLong
        );
        require!(
            outputs.signing_time.len() <= Attestation::MAX_STRING_LEN,
            ProvenanceError::StringTooLong
        );
        require!(
            outputs.cert_fingerprint.len() <= Attestation::MAX_STRING_LEN,
            ProvenanceError::StringTooLong
        );

        // 5. Store attestation from verified outputs
        let attestation = &mut ctx.accounts.attestation;
        attestation.content_hash = outputs.content_hash;
        attestation.has_c2pa = outputs.has_c2pa;
        attestation.trust_list_match = outputs.trust_list_match;
        attestation.validation_state = outputs.validation_state;
        attestation.digital_source_type = outputs.digital_source_type;
        attestation.issuer = outputs.issuer;
        attestation.common_name = outputs.common_name;
        attestation.software_agent = outputs.software_agent;
        attestation.signing_time = outputs.signing_time;
        attestation.cert_fingerprint = outputs.cert_fingerprint;
        attestation.submitted_by = ctx.accounts.submitter.key();
        attestation.timestamp = Clock::get()?.unix_timestamp;
        attestation.bump = ctx.bumps.attestation;

        msg!(
            "Attestation stored for content_hash: {:?}",
            hex::encode(content_hash)
        );

        Ok(())
    }
}

/// Parsed public outputs from the SP1 proof.
/// Mirrors `prover_shared::PublicOutputs` but defined locally to avoid
/// a cross-service dependency on the prover crate.
struct ParsedOutputs {
    content_hash: [u8; 32],
    has_c2pa: bool,
    trust_list_match: String,
    validation_state: String,
    digital_source_type: String,
    issuer: String,
    common_name: String,
    software_agent: String,
    signing_time: String,
    cert_fingerprint: String,
}

/// Parse bincode 1.x serialized PublicOutputs from SP1 public values.
///
/// Layout:
/// - `[u8; 32]`: 32 raw bytes (content_hash)
/// - `bool`: 1 byte (has_c2pa)
/// - 8x `String`: each is u64 LE length prefix + UTF-8 bytes
fn parse_public_outputs(data: &[u8]) -> Result<ParsedOutputs> {
    let mut cursor = 0usize;

    // content_hash: [u8; 32]
    if data.len() < cursor + 32 {
        return err!(ProvenanceError::InvalidPublicOutputs);
    }
    let mut content_hash = [0u8; 32];
    content_hash.copy_from_slice(&data[cursor..cursor + 32]);
    cursor += 32;

    // has_c2pa: bool (1 byte)
    if data.len() < cursor + 1 {
        return err!(ProvenanceError::InvalidPublicOutputs);
    }
    let has_c2pa = data[cursor] != 0;
    cursor += 1;

    // 8 String fields
    let trust_list_match = read_bincode_string(data, &mut cursor)?;
    let validation_state = read_bincode_string(data, &mut cursor)?;
    let digital_source_type = read_bincode_string(data, &mut cursor)?;
    let issuer = read_bincode_string(data, &mut cursor)?;
    let common_name = read_bincode_string(data, &mut cursor)?;
    let software_agent = read_bincode_string(data, &mut cursor)?;
    let signing_time = read_bincode_string(data, &mut cursor)?;
    let cert_fingerprint = read_bincode_string(data, &mut cursor)?;

    Ok(ParsedOutputs {
        content_hash,
        has_c2pa,
        trust_list_match,
        validation_state,
        digital_source_type,
        issuer,
        common_name,
        software_agent,
        signing_time,
        cert_fingerprint,
    })
}

/// Read a bincode 1.x String: u64 LE length prefix followed by UTF-8 bytes.
fn read_bincode_string(data: &[u8], cursor: &mut usize) -> Result<String> {
    if data.len() < *cursor + 8 {
        return err!(ProvenanceError::InvalidPublicOutputs);
    }
    let len_bytes: [u8; 8] = data[*cursor..*cursor + 8]
        .try_into()
        .map_err(|_| error!(ProvenanceError::InvalidPublicOutputs))?;
    let len = u64::from_le_bytes(len_bytes) as usize;
    *cursor += 8;

    if data.len() < *cursor + len {
        return err!(ProvenanceError::InvalidPublicOutputs);
    }
    let s = core::str::from_utf8(&data[*cursor..*cursor + len])
        .map_err(|_| error!(ProvenanceError::InvalidPublicOutputs))?
        .to_string();
    *cursor += len;

    Ok(s)
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
