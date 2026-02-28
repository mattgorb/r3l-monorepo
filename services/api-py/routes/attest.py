import asyncio
import hashlib

import httpx
from fastapi import APIRouter, File, Form, Header, HTTPException, UploadFile
from nacl.signing import VerifyKey
from nacl.exceptions import BadSignatureError
from pydantic import BaseModel
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
from storage import get_storage
import db
from solana_read import lookup_attestation
from solders.pubkey import Pubkey

router = APIRouter()

MAX_FILE_SIZE = 50 * 1024 * 1024  # 50 MB


# ── Shared helper ──────────────────────────────────────────────────

async def _submit_attestation(
    *,
    settings: Settings,
    content_hash_hex: str,
    verify_output: dict,
    tlsh_hash: str | None,
    clip_embedding: list[float] | None,
    content_type: str = "file",
    source_url: str | None = None,
    mime_type: str | None = None,
    content_size: int | None = None,
    stored: bool = False,
    wallet_pubkey: str | None = None,
    wallet_message: str | None = None,
    wallet_signature: str | None = None,
    privacy_mode: bool = False,
    private_mode: bool = False,
) -> dict:
    """Shared attestation: PDA derivation, idempotency, wallet verification, Solana tx, DB insert."""
    content_hash = bytes.fromhex(content_hash_hex)
    program_id = Pubkey.from_string(settings.program_id)
    pda, _ = find_pda([ATTESTATION_SEED, content_hash], program_id)

    # Idempotency
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

    # Wallet identity
    email_domain = ""
    email_hash = b"\x00" * 32
    wallet_bytes = b"\x00" * 32
    resolved_wallet = None
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
        try:
            verify_key = VerifyKey(pk_bytes)
            verify_key.verify(wallet_message.encode(), sig_bytes)
        except BadSignatureError:
            raise HTTPException(400, "invalid wallet signature")
        wallet_bytes = pk_bytes
        resolved_wallet = wallet_pubkey
        ed25519_ix = create_ed25519_instruction(pk_bytes, sig_bytes, wallet_message.encode())

    # Privacy mode: keep identity in Postgres but zero it out for Solana
    if privacy_mode:
        wallet_bytes = b"\x00" * 32
        ed25519_ix = None
        email_domain = ""
        email_hash = b"\x00" * 32

    # Versioning
    trust_hash = compute_trust_bundle_hash(settings.trust_dir)

    sig = None
    pda_str = str(pda)

    # Private mode: skip Solana entirely, only store in Postgres
    if not private_mode:
        # Encode instruction
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

        # Send Solana tx
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

    # DB insert
    await db.insert_attestation(
        content_hash=content_hash_hex,
        tx_signature=sig,
        pda=pda_str if not private_mode else None,
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
        wallet_pubkey=resolved_wallet,
        verifier_version=VERIFIER_VERSION,
        trust_bundle_hash=trust_hash,
        tlsh_hash=tlsh_hash,
        clip_embedding=clip_embedding,
        content_type=content_type,
        source_url=source_url,
        mime_type=mime_type,
        content_size=content_size,
        stored=stored,
        private=private_mode,
    )

    result = {
        "signature": sig,
        "attestation_pda": pda_str if not private_mode else None,
        "content_hash": content_hash_hex,
        "verify_output": verify_output,
        "private": private_mode,
    }
    if resolved_wallet:
        result["wallet_pubkey"] = resolved_wallet
    return result


# ── POST /api/attest (file upload) ────────────────────────────────

@router.post("/attest")
async def attest(
    file: UploadFile = File(...),
    store_content: str = Form("true"),
    private_mode: str = Form("false"),
    wallet_pubkey: str = Form(None),
    wallet_message: str = Form(None),
    wallet_signature: str = Form(None),
    x_api_key: str | None = Header(None),
):
    settings = Settings()
    caller = await db.get_customer_by_api_key(x_api_key) if x_api_key else None
    file_bytes = await file.read()
    validate_upload(file_bytes, file.content_type)

    # Compute similarity hashes
    file_tlsh = compute_tlsh(file_bytes)
    file_clip = compute_clip_embedding(file_bytes, file.content_type)

    # Verify file (C2PA extraction)
    verify_output = await run_verifier(file_bytes, file.filename or "upload", settings)

    content_hash_hex = verify_output.get("content_hash")
    if not content_hash_hex:
        raise HTTPException(500, "no content hash from verifier")

    # Store content
    should_store = store_content.lower() not in ("false", "0", "no")
    if should_store:
        storage = get_storage()
        ct = file.content_type or "application/octet-stream"
        await storage.save(content_hash_hex, file_bytes, ct)

    is_private = private_mode.lower() not in ("false", "0", "no")

    return await _submit_attestation(
        settings=settings,
        content_hash_hex=content_hash_hex,
        verify_output=verify_output,
        tlsh_hash=file_tlsh,
        clip_embedding=file_clip,
        content_type="file",
        mime_type=file.content_type,
        content_size=len(file_bytes),
        stored=should_store,
        wallet_pubkey=wallet_pubkey,
        wallet_message=wallet_message,
        wallet_signature=wallet_signature,
        privacy_mode=caller.get("privacy_mode", False) if caller else False,
        private_mode=is_private,
    )


# ── POST /api/attest/url ──────────────────────────────────────────

class AttestUrlRequest(BaseModel):
    url: str
    store_content: bool = True
    private_mode: bool = False
    headers: dict[str, str] | None = None  # optional auth headers forwarded to the target URL


@router.post("/attest/url")
async def attest_url(req: AttestUrlRequest, x_api_key: str | None = Header(None)):
    settings = Settings()
    caller = await db.get_customer_by_api_key(x_api_key) if x_api_key else None

    # Build fetch headers — always include User-Agent, merge caller-provided headers
    fetch_headers = {"User-Agent": "R3L-Attester/1.0"}
    if req.headers:
        fetch_headers.update(req.headers)

    # Fetch URL
    try:
        async with httpx.AsyncClient(follow_redirects=True, timeout=30) as client:
            resp = await client.get(req.url, headers=fetch_headers)
            resp.raise_for_status()
    except httpx.HTTPStatusError as e:
        raise HTTPException(502, f"URL returned {e.response.status_code}")
    except Exception as e:
        raise HTTPException(502, f"failed to fetch URL: {e}")

    page_bytes = resp.content
    if len(page_bytes) > MAX_FILE_SIZE:
        raise HTTPException(413, "page too large")

    content_type_header = resp.headers.get("content-type", "text/html").split(";")[0].strip()

    # Hash + similarity
    content_hash_hex = hashlib.sha256(page_bytes).hexdigest()
    file_tlsh = compute_tlsh(page_bytes)
    file_clip = compute_clip_embedding(page_bytes, content_type_header)

    # Store
    if req.store_content:
        storage = get_storage()
        await storage.save(content_hash_hex, page_bytes, content_type_header)

    # No C2PA for URLs
    verify_output = {
        "content_hash": content_hash_hex,
        "has_c2pa": False,
    }

    return await _submit_attestation(
        settings=settings,
        content_hash_hex=content_hash_hex,
        verify_output=verify_output,
        tlsh_hash=file_tlsh,
        clip_embedding=file_clip,
        content_type="url",
        source_url=req.url,
        mime_type=content_type_header,
        content_size=len(page_bytes),
        stored=req.store_content,
        privacy_mode=caller.get("privacy_mode", False) if caller else False,
        private_mode=req.private_mode,
    )


# ── POST /api/attest/text ─────────────────────────────────────────

class AttestTextRequest(BaseModel):
    text: str
    title: str | None = None
    store_content: bool = True
    private_mode: bool = False


@router.post("/attest/text")
async def attest_text(req: AttestTextRequest, x_api_key: str | None = Header(None)):
    settings = Settings()
    caller = await db.get_customer_by_api_key(x_api_key) if x_api_key else None

    text_bytes = req.text.encode("utf-8")
    if len(text_bytes) > MAX_FILE_SIZE:
        raise HTTPException(413, "text too large")

    content_hash_hex = hashlib.sha256(text_bytes).hexdigest()
    file_tlsh = compute_tlsh(text_bytes)
    file_clip = compute_clip_embedding(text_bytes, "text/plain")

    if req.store_content:
        storage = get_storage()
        await storage.save(content_hash_hex, text_bytes, "text/plain")

    verify_output = {
        "content_hash": content_hash_hex,
        "has_c2pa": False,
    }

    return await _submit_attestation(
        settings=settings,
        content_hash_hex=content_hash_hex,
        verify_output=verify_output,
        tlsh_hash=file_tlsh,
        clip_embedding=file_clip,
        content_type="text",
        mime_type="text/plain",
        content_size=len(text_bytes),
        stored=req.store_content,
        privacy_mode=caller.get("privacy_mode", False) if caller else False,
        private_mode=req.private_mode,
    )
