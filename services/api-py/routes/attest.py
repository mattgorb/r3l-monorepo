import asyncio

from fastapi import APIRouter, File, Form, HTTPException, UploadFile
from nacl.signing import VerifyKey
from nacl.exceptions import BadSignatureError
import base58

from config import Settings
from similarity import compute_tlsh, compute_clip_embedding
from routes.verify import run_verifier, validate_upload
from versioning import VERIFIER_VERSION, compute_trust_bundle_hash
from solana_tx import (
    ATTESTATION_SEED,
    build_and_send_tx,
    create_ed25519_instruction,
    encode_attestation_data,
    find_pda,
)
import db
from solana_read import lookup_attestation
from solders.pubkey import Pubkey

router = APIRouter()


@router.post("/attest")
async def attest(
    file: UploadFile = File(...),
    wallet_pubkey: str = Form(None),
    wallet_message: str = Form(None),
    wallet_signature: str = Form(None),
):
    settings = Settings()
    file_bytes = await file.read()
    validate_upload(file_bytes, file.content_type)

    # 0. Compute similarity hashes (TLSH + CLIP)
    file_tlsh = compute_tlsh(file_bytes)
    file_clip = compute_clip_embedding(file_bytes, file.content_type)

    # 1. Verify file (C2PA extraction)
    verify_output = await run_verifier(file_bytes, file.filename or "upload", settings)

    # 2. Extract content hash
    content_hash_hex = verify_output.get("content_hash")
    if not content_hash_hex:
        raise HTTPException(500, "no content hash from verifier")
    content_hash = bytes.fromhex(content_hash_hex)

    # 3. Derive PDA
    program_id = Pubkey.from_string(settings.program_id)
    pda, _ = find_pda([ATTESTATION_SEED, content_hash], program_id)

    # 4. Idempotency — check if attestation already exists
    existing = await asyncio.to_thread(
        lookup_attestation, settings.solana_rpc_url, settings.program_id, content_hash_hex
    )
    if existing:
        return {
            "signature": None,
            "attestation_pda": str(pda),
            "content_hash": content_hash_hex,
            "verify_output": verify_output,
            "existing": True,
        }

    # 5. Email identity (no longer per-file — handled via account auth)
    email_domain = ""
    email_hash = b"\x00" * 32

    # 6. Resolve optional wallet identity
    wallet_bytes = b"\x00" * 32
    resolved_wallet_pubkey = None
    ed25519_ix = None
    if wallet_pubkey and wallet_message and wallet_signature:
        if content_hash_hex not in wallet_message:
            raise HTTPException(400, "wallet message must contain the content hash")

        try:
            pk_bytes = base58.b58decode(wallet_pubkey)
            sig_bytes = base58.b58decode(wallet_signature)
        except Exception:
            raise HTTPException(400, "invalid base58 encoding")

        if len(pk_bytes) != 32:
            raise HTTPException(400, "invalid wallet pubkey length")

        # Off-chain verification (fast-fail for bad signatures)
        try:
            verify_key = VerifyKey(pk_bytes)
            verify_key.verify(wallet_message.encode(), sig_bytes)
        except BadSignatureError:
            raise HTTPException(400, "invalid wallet signature")

        wallet_bytes = pk_bytes
        resolved_wallet_pubkey = wallet_pubkey

        # Build Ed25519 precompile instruction for on-chain verification
        ed25519_ix = create_ed25519_instruction(pk_bytes, sig_bytes, wallet_message.encode())

    # 7. Compute versioning
    trust_hash = compute_trust_bundle_hash(settings.trust_dir)

    # 8. Encode unified instruction
    ix_data = encode_attestation_data(
        content_hash=content_hash,
        has_c2pa=verify_output.get("has_c2pa", False),
        trust_list_match=verify_output.get("trust_list_match") or "",
        validation_state=verify_output.get("validation_state") or "",
        digital_source_type=verify_output.get("digital_source_type") or "",
        issuer=verify_output.get("issuer") or "",
        common_name=verify_output.get("common_name") or "",
        software_agent=verify_output.get("software_agent") or "",
        signing_time=verify_output.get("signing_time") or "",
        cert_fingerprint=verify_output.get("cert_fingerprint") or "",
        email_domain=email_domain,
        email_hash=email_hash,
        wallet=wallet_bytes,
        verifier_version=VERIFIER_VERSION,
        trust_bundle_hash=trust_hash,
    )

    # 9. Send single Solana transaction
    extra_ixs = [ed25519_ix] if ed25519_ix else None
    sig, pda_str = await asyncio.to_thread(
        build_and_send_tx,
        settings.solana_rpc_url,
        settings.solana_keypair_path,
        settings.program_id,
        ix_data,
        pda,
        200_000,
        extra_ixs,
    )

    # 10. Write single DB row
    await db.insert_attestation(
        content_hash=content_hash_hex,
        tx_signature=sig,
        pda=pda_str,
        has_c2pa=verify_output.get("has_c2pa", False),
        trust_list_match=verify_output.get("trust_list_match") or "",
        validation_state=verify_output.get("validation_state") or "",
        digital_source_type=verify_output.get("digital_source_type") or "",
        issuer=verify_output.get("issuer") or "",
        common_name=verify_output.get("common_name") or "",
        software_agent=verify_output.get("software_agent") or "",
        signing_time=verify_output.get("signing_time") or "",
        cert_fingerprint=verify_output.get("cert_fingerprint") or "",
        email_domain=email_domain or None,
        wallet_pubkey=resolved_wallet_pubkey,
        verifier_version=VERIFIER_VERSION,
        trust_bundle_hash=trust_hash,
        tlsh_hash=file_tlsh,
        clip_embedding=file_clip,
    )

    result = {
        "signature": sig,
        "attestation_pda": pda_str,
        "content_hash": content_hash_hex,
        "verify_output": verify_output,
    }
    if email_domain:
        result["email_domain"] = email_domain
    if resolved_wallet_pubkey:
        result["wallet_pubkey"] = resolved_wallet_pubkey

    return result
