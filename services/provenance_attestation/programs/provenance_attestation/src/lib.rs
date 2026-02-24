use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::instructions as ix_sysvar;
use anchor_lang::solana_program::ed25519_program;

mod constants;
mod errors;
mod state;

use constants::ATTESTATION_SEED;
#[cfg(not(feature = "skip-authority-check"))]
use constants::AUTHORITY;
#[cfg(not(feature = "skip-verification"))]
use constants::SP1_VKEY_HASH;
use errors::ProvenanceError;
use state::Attestation;
#[cfg(not(feature = "skip-authority-check"))]
use anchor_lang::solana_program::pubkey::Pubkey as SolPubkey;
#[cfg(not(feature = "skip-authority-check"))]
use core::str::FromStr;

declare_id!("63jq6M3t5NafYWcADqLDCLnhd5qPfEmCUcaA9iWh5YWz");

#[program]
pub mod provenance_attestation {
    use super::*;

    /// Submit a ZK proof to store a provenance attestation.
    /// Anyone can call this — the Groth16 proof is the authorization.
    ///
    /// The `public_inputs` bytes are bincode-serialized PublicOutputs from the SP1 guest.
    /// After proof verification, we parse them on-chain and store directly.
    /// The `content_hash` arg is only for PDA seed derivation
    /// and is verified against the parsed public outputs.
    ///
    /// Optional identity fields (email, wallet) and versioning are passed as extra args.
    pub fn submit_proof(
        ctx: Context<SubmitProof>,
        proof: Vec<u8>,
        public_inputs: Vec<u8>,
        content_hash: [u8; 32],
        email_domain: String,
        email_hash: [u8; 32],
        wallet: Pubkey,
        verifier_version: String,
        trust_bundle_hash: String,
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
        require!(outputs.trust_list_match.len() <= Attestation::MAX_STRING_LEN, ProvenanceError::StringTooLong);
        require!(outputs.validation_state.len() <= Attestation::MAX_STRING_LEN, ProvenanceError::StringTooLong);
        require!(outputs.digital_source_type.len() <= Attestation::MAX_STRING_LEN, ProvenanceError::StringTooLong);
        require!(outputs.issuer.len() <= Attestation::MAX_STRING_LEN, ProvenanceError::StringTooLong);
        require!(outputs.common_name.len() <= Attestation::MAX_STRING_LEN, ProvenanceError::StringTooLong);
        require!(outputs.software_agent.len() <= Attestation::MAX_STRING_LEN, ProvenanceError::StringTooLong);
        require!(outputs.signing_time.len() <= Attestation::MAX_STRING_LEN, ProvenanceError::StringTooLong);
        require!(outputs.cert_fingerprint.len() <= Attestation::MAX_STRING_LEN, ProvenanceError::StringTooLong);
        require!(email_domain.len() <= Attestation::MAX_STRING_LEN, ProvenanceError::StringTooLong);
        require!(verifier_version.len() <= Attestation::MAX_STRING_LEN, ProvenanceError::StringTooLong);
        require!(trust_bundle_hash.len() <= Attestation::MAX_STRING_LEN, ProvenanceError::StringTooLong);

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
        attestation.proof_type = "zk_groth16".to_string();
        attestation.email_domain = email_domain;
        attestation.email_hash = email_hash;
        attestation.wallet = wallet;
        attestation.verifier_version = verifier_version;
        attestation.trust_bundle_hash = trust_bundle_hash;

        // Verify wallet signature on-chain via Ed25519 precompile
        if wallet != Pubkey::default() {
            let sig = verify_wallet_sig(&ctx.accounts.instructions, &wallet, &content_hash)?;
            attestation.wallet_sig = sig;
        }

        msg!(
            "Attestation stored for content_hash: {:?}",
            hex::encode(content_hash)
        );

        Ok(())
    }

    /// Submit an attestation directly as the trusted R3L verifier.
    /// Authority-gated: only the R3L server keypair can call this.
    /// No ZK proof needed — the server has already verified the file off-chain.
    ///
    /// Includes optional identity fields (email, wallet) and versioning.
    pub fn submit_attestation(
        ctx: Context<SubmitAttestation>,
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
        email_domain: String,
        email_hash: [u8; 32],
        wallet: Pubkey,
        verifier_version: String,
        trust_bundle_hash: String,
    ) -> Result<()> {
        // 1. Verify authority
        #[cfg(not(feature = "skip-authority-check"))]
        {
            let expected = SolPubkey::from_str(AUTHORITY)
                .map_err(|_| ProvenanceError::Unauthorized)?;
            require!(
                ctx.accounts.authority.key() == expected,
                ProvenanceError::Unauthorized
            );
        }

        // 2. Validate string lengths
        require!(trust_list_match.len() <= Attestation::MAX_STRING_LEN, ProvenanceError::StringTooLong);
        require!(validation_state.len() <= Attestation::MAX_STRING_LEN, ProvenanceError::StringTooLong);
        require!(digital_source_type.len() <= Attestation::MAX_STRING_LEN, ProvenanceError::StringTooLong);
        require!(issuer.len() <= Attestation::MAX_STRING_LEN, ProvenanceError::StringTooLong);
        require!(common_name.len() <= Attestation::MAX_STRING_LEN, ProvenanceError::StringTooLong);
        require!(software_agent.len() <= Attestation::MAX_STRING_LEN, ProvenanceError::StringTooLong);
        require!(signing_time.len() <= Attestation::MAX_STRING_LEN, ProvenanceError::StringTooLong);
        require!(cert_fingerprint.len() <= Attestation::MAX_STRING_LEN, ProvenanceError::StringTooLong);
        require!(email_domain.len() <= Attestation::MAX_STRING_LEN, ProvenanceError::StringTooLong);
        require!(verifier_version.len() <= Attestation::MAX_STRING_LEN, ProvenanceError::StringTooLong);
        require!(trust_bundle_hash.len() <= Attestation::MAX_STRING_LEN, ProvenanceError::StringTooLong);

        // 3. Store attestation
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
        attestation.cert_fingerprint = cert_fingerprint;
        attestation.proof_type = "trusted_verifier".to_string();
        attestation.submitted_by = ctx.accounts.authority.key();
        attestation.timestamp = Clock::get()?.unix_timestamp;
        attestation.bump = ctx.bumps.attestation;
        attestation.email_domain = email_domain;
        attestation.email_hash = email_hash;
        attestation.wallet = wallet;
        attestation.verifier_version = verifier_version;
        attestation.trust_bundle_hash = trust_bundle_hash;

        // Verify wallet signature on-chain via Ed25519 precompile
        if wallet != Pubkey::default() {
            let sig = verify_wallet_sig(&ctx.accounts.instructions, &wallet, &content_hash)?;
            attestation.wallet_sig = sig;
        }

        msg!(
            "Trusted attestation stored for content_hash: {:?}",
            hex::encode(content_hash),
        );

        Ok(())
    }
}

const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";

/// Verify that the transaction contains an Ed25519 precompile instruction
/// with the expected wallet pubkey and message ("R3L: attest " + hex(content_hash)).
/// Returns the 64-byte signature extracted from the instruction data.
fn verify_wallet_sig(
    instructions_account: &UncheckedAccount,
    wallet: &Pubkey,
    content_hash: &[u8; 32],
) -> Result<[u8; 64]> {
    let ix_sysvar_data = instructions_account.try_borrow_data()
        .map_err(|_| error!(ProvenanceError::InvalidWalletSigVerify))?;

    // Read number of instructions from the sysvar (last 2 bytes = u16 LE)
    let num_ix = if ix_sysvar_data.len() >= 2 {
        u16::from_le_bytes([
            ix_sysvar_data[ix_sysvar_data.len() - 2],
            ix_sysvar_data[ix_sysvar_data.len() - 1],
        ]) as usize
    } else {
        return err!(ProvenanceError::InvalidWalletSigVerify);
    };

    // Scan transaction instructions for the Ed25519 precompile
    // Use load_instruction_at_checked from the sysvar
    drop(ix_sysvar_data); // release borrow before calling sysvar functions

    for i in 0..num_ix {
        let ix = match ix_sysvar::load_instruction_at_checked(i, &instructions_account.to_account_info()) {
            Ok(ix) => ix,
            Err(_) => continue,
        };

        if ix.program_id != ed25519_program::ID {
            continue;
        }

        // Ed25519 instruction data layout (1 signature):
        // [0..2]:   num_signatures (u8) + padding (u8)
        // [2..16]:  Ed25519SignatureOffsets struct (7 x u16 LE)
        // [16..80]: 64-byte signature
        // [80..112]: 32-byte pubkey
        // [112..]:  message bytes
        let data = &ix.data;
        if data.len() < 112 {
            continue;
        }

        // Extract signature (bytes 16..80)
        let mut sig = [0u8; 64];
        sig.copy_from_slice(&data[16..80]);

        // Extract pubkey (bytes 80..112)
        let ix_pubkey = Pubkey::try_from(&data[80..112])
            .map_err(|_| error!(ProvenanceError::InvalidWalletSigVerify))?;

        // Verify pubkey matches the wallet parameter
        require!(ix_pubkey == *wallet, ProvenanceError::WalletPubkeyMismatch);

        // Extract and verify message
        let message = &data[112..];
        require!(
            verify_wallet_message(message, content_hash),
            ProvenanceError::InvalidWalletSigVerify
        );

        return Ok(sig);
    }

    // No Ed25519 instruction found
    err!(ProvenanceError::InvalidWalletSigVerify)
}

/// Verify that a message matches "R3L: attest " + hex(content_hash)
fn verify_wallet_message(message: &[u8], content_hash: &[u8; 32]) -> bool {
    let prefix = b"R3L: attest ";
    if message.len() != prefix.len() + 64 {
        return false;
    }
    if &message[..prefix.len()] != prefix {
        return false;
    }
    let hex_part = &message[prefix.len()..];
    for (i, byte) in content_hash.iter().enumerate() {
        if hex_part[i * 2] != HEX_CHARS[(byte >> 4) as usize] {
            return false;
        }
        if hex_part[i * 2 + 1] != HEX_CHARS[(byte & 0x0f) as usize] {
            return false;
        }
    }
    true
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
    /// CHECK: Instructions sysvar for Ed25519 signature verification
    #[account(address = ix_sysvar::ID)]
    pub instructions: UncheckedAccount<'info>,
}

#[derive(Accounts)]
#[instruction(content_hash: [u8; 32])]
pub struct SubmitAttestation<'info> {
    #[account(
        init,
        payer = authority,
        space = Attestation::SPACE,
        seeds = [ATTESTATION_SEED, content_hash.as_ref()],
        bump,
    )]
    pub attestation: Account<'info, Attestation>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
    /// CHECK: Instructions sysvar for Ed25519 signature verification
    #[account(address = ix_sysvar::ID)]
    pub instructions: UncheckedAccount<'info>,
}
