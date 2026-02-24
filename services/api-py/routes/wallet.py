import asyncio

from fastapi import APIRouter, HTTPException
from pydantic import BaseModel
from nacl.signing import VerifyKey
from nacl.exceptions import BadSignatureError
import base58

from config import Settings
from solana_tx import (
    WALLET_SEED,
    build_and_send_tx,
    encode_wallet_data,
    find_pda,
)
import db
from solders.pubkey import Pubkey

router = APIRouter()


class WalletAttestRequest(BaseModel):
    content_hash: str
    pubkey: str
    message: str
    signature: str


@router.post("/attest")
async def attest_wallet(req: WalletAttestRequest):
    settings = Settings()

    # 1. Validate content hash
    try:
        content_hash_bytes = bytes.fromhex(req.content_hash)
    except ValueError:
        raise HTTPException(400, "invalid content hash hex")
    if len(content_hash_bytes) != 32:
        raise HTTPException(400, "content hash must be 32 bytes")

    # 2. Validate message contains the content hash (prevents cross-file replay)
    if req.content_hash not in req.message:
        raise HTTPException(400, "message must contain the content hash")

    # 3. Verify Ed25519 signature
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

    # 4. Derive PDA
    wallet_pubkey = Pubkey.from_string(req.pubkey)
    program_id = Pubkey.from_string(settings.program_id)
    pda, _ = find_pda([WALLET_SEED, content_hash_bytes, bytes(wallet_pubkey)], program_id)

    # 5. Encode + send Solana tx
    ix_data = encode_wallet_data(content_hash_bytes, wallet_pubkey)

    sig, pda_str = await asyncio.to_thread(
        build_and_send_tx,
        settings.solana_rpc_url,
        settings.solana_keypair_path,
        settings.program_id,
        ix_data,
        pda,
        200_000,
    )

    # 6. Write to Postgres
    await db.insert_attestation(
        content_hash=req.content_hash,
        kind="wallet",
        proof_type="wallet_signature",
        tx_signature=sig,
        pda=pda_str,
        wallet_pubkey=req.pubkey,
    )

    return {
        "signature": sig,
        "wallet_pda": pda_str,
        "content_hash": req.content_hash,
        "pubkey": req.pubkey,
    }
