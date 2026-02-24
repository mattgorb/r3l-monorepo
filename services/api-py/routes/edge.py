import asyncio
import secrets

from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel
from nacl.signing import VerifyKey
from nacl.exceptions import BadSignatureError
import base58

from auth import require_api_key
from config import Settings
from versioning import VERIFIER_VERSION, compute_trust_bundle_hash
from solana_tx import (
    ATTESTATION_SEED,
    build_and_send_tx,
    create_ed25519_instruction,
    encode_attestation_data,
    find_pda,
)
from solana_read import lookup_attestation
import db
from solders.pubkey import Pubkey

router = APIRouter()


class EdgeAttestRequest(BaseModel):
    content_hash: str
    has_c2pa: bool = False
    trust_list_match: str = ""
    validation_state: str = ""
    digital_source_type: str = ""
    issuer: str = ""
    common_name: str = ""
    software_agent: str = ""
    signing_time: str = ""
    cert_fingerprint: str = ""
    wallet_signature: str = ""  # base58 Ed25519 sig for on-chain verification
    tlsh_hash: str = ""         # edge-computed TLSH hash
    clip_embedding: list[float] = []  # edge-computed CLIP embedding (512-dim)


class RegisterRequest(BaseModel):
    pubkey: str
    message: str
    signature: str
    name: str = ""


@router.post("/register")
async def register(req: RegisterRequest):
    # 1. Verify Ed25519 signature proves wallet ownership
    try:
        pubkey_bytes = base58.b58decode(req.pubkey)
        sig_bytes = base58.b58decode(req.signature)
    except Exception:
        raise HTTPException(400, "invalid base58 encoding")

    if len(pubkey_bytes) != 32:
        raise HTTPException(400, "invalid pubkey length")

    try:
        verify_key = VerifyKey(pubkey_bytes)
        verify_key.verify(req.message.encode(), sig_bytes)
    except BadSignatureError:
        raise HTTPException(400, "invalid signature")

    # 2. Check if wallet is already registered
    existing = await db.get_customer_by_wallet(req.pubkey)
    if existing:
        return {"api_key": existing["api_key"], "pubkey": req.pubkey, "name": existing["name"]}

    # 3. Create new customer tied to wallet
    api_key = "r3l_" + secrets.token_hex(24)
    name = req.name or req.pubkey[:8]
    customer = await db.insert_customer(name=name, api_key=api_key, wallet_pubkey=req.pubkey, auth_method="wallet")
    return {"api_key": customer["api_key"], "pubkey": req.pubkey, "name": customer["name"]}


@router.post("/attest")
async def edge_attest(req: EdgeAttestRequest, customer: dict = Depends(require_api_key)):
    settings = Settings()

    # 1. Validate content hash
    try:
        content_hash_bytes = bytes.fromhex(req.content_hash)
    except ValueError:
        raise HTTPException(400, "invalid content hash hex")
    if len(content_hash_bytes) != 32:
        raise HTTPException(400, "content hash must be 32 bytes")

    program_id = Pubkey.from_string(settings.program_id)

    # 2. Idempotency — check if attestation already exists
    existing = await asyncio.to_thread(
        lookup_attestation, settings.solana_rpc_url, settings.program_id, req.content_hash
    )
    if existing:
        pda, _ = find_pda([ATTESTATION_SEED, content_hash_bytes], program_id)
        return {
            "signature": None,
            "attestation_pda": str(pda),
            "content_hash": req.content_hash,
            "existing": True,
        }

    # 3. Resolve wallet from customer record (only if signature provided)
    wallet_bytes = b"\x00" * 32
    wallet_pubkey = None
    ed25519_ix = None
    customer_wallet = customer.get("wallet_pubkey")
    if customer_wallet and req.wallet_signature:
        try:
            pk_bytes = base58.b58decode(customer_wallet)
            sig_bytes = base58.b58decode(req.wallet_signature)
        except Exception:
            raise HTTPException(400, "invalid base58 encoding in wallet_signature")

        # Verify signature off-chain first (fast-fail)
        wallet_message = f"R3L: attest {req.content_hash}"
        try:
            verify_key = VerifyKey(pk_bytes)
            verify_key.verify(wallet_message.encode(), sig_bytes)
        except BadSignatureError:
            raise HTTPException(400, "invalid wallet signature")

        wallet_bytes = pk_bytes
        wallet_pubkey = customer_wallet
        ed25519_ix = create_ed25519_instruction(pk_bytes, sig_bytes, wallet_message.encode())

    # 4. Compute versioning
    trust_hash = compute_trust_bundle_hash(settings.trust_dir)

    # 5. Encode unified instruction (single tx)
    pda, _ = find_pda([ATTESTATION_SEED, content_hash_bytes], program_id)

    ix_data = encode_attestation_data(
        content_hash=content_hash_bytes,
        has_c2pa=req.has_c2pa,
        trust_list_match=req.trust_list_match,
        validation_state=req.validation_state,
        digital_source_type=req.digital_source_type,
        issuer=req.issuer,
        common_name=req.common_name,
        software_agent=req.software_agent,
        signing_time=req.signing_time,
        cert_fingerprint=req.cert_fingerprint,
        wallet=wallet_bytes,
        verifier_version=VERIFIER_VERSION,
        trust_bundle_hash=trust_hash,
    )

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

    # 6. Single DB insert (include org info if caller is an org key)
    org_id = customer.get("org_id") if customer.get("type") == "org" else None
    org_domain = customer.get("org_domain") if customer.get("type") == "org" else None

    await db.insert_attestation(
        content_hash=req.content_hash,
        tx_signature=sig,
        pda=pda_str,
        has_c2pa=req.has_c2pa,
        trust_list_match=req.trust_list_match,
        validation_state=req.validation_state,
        digital_source_type=req.digital_source_type,
        issuer=req.issuer,
        common_name=req.common_name,
        software_agent=req.software_agent,
        signing_time=req.signing_time,
        cert_fingerprint=req.cert_fingerprint,
        wallet_pubkey=wallet_pubkey,
        submitted_by=customer.get("name") or org_domain,
        verifier_version=VERIFIER_VERSION,
        trust_bundle_hash=trust_hash,
        tlsh_hash=req.tlsh_hash or None,
        clip_embedding=req.clip_embedding or None,
        org_id=org_id,
        org_domain=org_domain,
    )

    result = {
        "signature": sig,
        "attestation_pda": pda_str,
        "content_hash": req.content_hash,
    }
    if wallet_pubkey:
        result["wallet_pubkey"] = wallet_pubkey

    return result
