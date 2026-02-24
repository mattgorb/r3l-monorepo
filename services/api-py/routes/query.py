import asyncio
from enum import Enum

from fastapi import APIRouter, HTTPException
from pydantic import BaseModel

from config import Settings
import db
from solana_read import lookup_attestation

router = APIRouter()

# ── Source type labels ────────────────────────────────────────────

SOURCE_TYPE_LABELS = {
    "digitalCapture": "Digital Capture (Camera/Device)",
    "trainedAlgorithmicMedia": "AI-Generated",
    "compositeWithTrainedAlgorithmicMedia": "Composite (includes AI)",
    "algorithmicMedia": "Algorithmically Generated",
    "digitalArt": "Digital Art",
    "compositeCapture": "Composite Capture",
}


def _source_label(raw: str | None) -> str | None:
    if not raw:
        return None
    for key, label in SOURCE_TYPE_LABELS.items():
        if key in raw:
            return label
    return raw


# ── Verdict computation ───────────────────────────────────────────


def _compute_verdict(att: dict) -> str:
    trust = att.get("trust_list_match", "")
    if trust in ("official", "curated"):
        return "trusted"
    if att.get("has_c2pa"):
        return "attested"
    if att.get("wallet_pubkey") or att.get("email_domain"):
        return "attested"
    return "attested"  # on-chain = attested at minimum


def _format_response(att: dict) -> dict:
    trust = att.get("trust_list_match", "")
    wallet = att.get("wallet_pubkey")
    wallet_sig = att.get("wallet_sig")

    return {
        "version": "1.0",
        "content_hash": att["content_hash"],
        "verdict": _compute_verdict(att),
        "c2pa": {
            "present": att.get("has_c2pa", False),
            "trust_list": trust or None,
            "validation_state": att.get("validation_state") or None,
            "digital_source_type": att.get("digital_source_type") or None,
            "source_type_label": _source_label(att.get("digital_source_type")),
        },
        "identity": {
            "email_domain": att.get("email_domain") or None,
            "wallet_pubkey": wallet or None,
            "wallet_verified_onchain": bool(wallet and wallet_sig),
            "org_domain": att.get("org_domain") or None,
        },
        "signer": {
            "issuer": att.get("issuer") or None,
            "common_name": att.get("common_name") or None,
            "software_agent": att.get("software_agent") or None,
            "signing_time": att.get("signing_time") or None,
        },
        "proof": {
            "type": att.get("proof_type", "trusted_verifier"),
            "on_chain": True,
            "pda": att.get("pda") or None,
            "tx_signature": att.get("tx_signature") or None,
        },
        "attestation": {
            "timestamp": att.get("timestamp") or att.get("created_at"),
            "submitted_by": att.get("submitted_by") or None,
            "verifier_version": att.get("verifier_version") or None,
            "trust_bundle_hash": att.get("trust_bundle_hash") or None,
        },
    }


# ── Endpoints ─────────────────────────────────────────────────────


@router.get("/v1/query/{content_hash}")
async def query(content_hash: str):
    """
    Structured trust verdict for a content hash.
    Designed for external consumers, AI agents, and integrations.
    """
    # DB first
    row = await db.get_attestation(content_hash)
    if row:
        return _format_response(row)

    # On-chain fallback
    settings = Settings()
    att = await asyncio.to_thread(
        lookup_attestation, settings.solana_rpc_url, settings.program_id, content_hash
    )
    if att is None:
        raise HTTPException(404, detail={
            "version": "1.0",
            "content_hash": content_hash,
            "verdict": "unknown",
            "error": "no attestation found",
        })
    return _format_response(att)


@router.post("/v1/query/batch")
async def query_batch(hashes: list[str]):
    """
    Batch query for multiple content hashes. Returns a list of verdicts.
    Max 50 hashes per request.
    """
    if len(hashes) > 50:
        raise HTTPException(400, "max 50 hashes per batch request")

    settings = Settings()
    results = []
    for h in hashes:
        row = await db.get_attestation(h)
        if row:
            results.append(_format_response(row))
            continue

        att = await asyncio.to_thread(
            lookup_attestation, settings.solana_rpc_url, settings.program_id, h
        )
        if att:
            results.append(_format_response(att))
        else:
            results.append({
                "version": "1.0",
                "content_hash": h,
                "verdict": "unknown",
            })

    return results
