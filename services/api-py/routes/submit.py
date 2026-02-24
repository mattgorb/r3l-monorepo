import asyncio

from fastapi import APIRouter, HTTPException
from pydantic import BaseModel

from config import Settings
from solana_tx import (
    ATTESTATION_SEED,
    build_and_send_tx,
    encode_proof_data,
    find_pda,
)
from solders.pubkey import Pubkey

router = APIRouter()


class SubmitRequest(BaseModel):
    content_hash: str
    proof: str
    public_inputs: str


@router.post("/submit")
async def submit(req: SubmitRequest):
    settings = Settings()

    content_hash_bytes = bytes.fromhex(req.content_hash)
    if len(content_hash_bytes) != 32:
        raise HTTPException(400, "content_hash must be 32 bytes")

    proof_bytes = bytes.fromhex(req.proof)
    public_inputs_bytes = bytes.fromhex(req.public_inputs)

    program_id = Pubkey.from_string(settings.program_id)
    pda, _ = find_pda([ATTESTATION_SEED, content_hash_bytes], program_id)

    ix_data = encode_proof_data(proof_bytes, public_inputs_bytes, content_hash_bytes)

    sig, pda_str = await asyncio.to_thread(
        build_and_send_tx,
        settings.solana_rpc_url,
        settings.solana_keypair_path,
        settings.program_id,
        ix_data,
        pda,
        400_000,
    )

    return {
        "signature": sig,
        "attestation_pda": pda_str,
    }
