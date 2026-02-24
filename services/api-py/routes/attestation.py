import asyncio

from fastapi import APIRouter, HTTPException

from config import Settings
import db
from solana_read import lookup_attestation

router = APIRouter()


@router.get("/attestation/{hash}")
async def lookup(hash: str):
    # Try DB first
    row = await db.get_attestation(hash)
    if row:
        result = {
            "content_hash": row["content_hash"],
            "has_c2pa": row["has_c2pa"],
            "trust_list_match": row["trust_list_match"] or "",
            "validation_state": row["validation_state"] or "",
            "digital_source_type": row["digital_source_type"] or "",
            "issuer": row["issuer"] or "",
            "common_name": row["common_name"] or "",
            "software_agent": row["software_agent"] or "",
            "signing_time": row["signing_time"] or "",
            "cert_fingerprint": row["cert_fingerprint"] or "",
            "submitted_by": row["submitted_by"] or "",
            "timestamp": row["created_at"],
            "proof_type": row["proof_type"],
        }
        if row.get("email_domain"):
            result["email_domain"] = row["email_domain"]
        if row.get("wallet_pubkey"):
            result["wallet_pubkey"] = row["wallet_pubkey"]
        if row.get("verifier_version"):
            result["verifier_version"] = row["verifier_version"]
        if row.get("trust_bundle_hash"):
            result["trust_bundle_hash"] = row["trust_bundle_hash"]
        if row.get("org_domain"):
            result["org_domain"] = row["org_domain"]
        return result

    # Fall back to on-chain lookup
    settings = Settings()
    result = await asyncio.to_thread(
        lookup_attestation, settings.solana_rpc_url, settings.program_id, hash
    )
    if result is None:
        raise HTTPException(404, "attestation not found")
    return result


@router.get("/attestations")
async def list_all():
    rows = await db.list_attestations()
    items = []
    for row in rows:
        item = {
            "content_hash": row["content_hash"],
            "proof_type": row["proof_type"],
            "timestamp": row["created_at"],
        }
        if row.get("issuer"):
            item["issuer"] = row["issuer"]
        if row.get("trust_list_match"):
            item["trust_list_match"] = row["trust_list_match"]
        if row.get("email_domain"):
            item["email_domain"] = row["email_domain"]
        if row.get("wallet_pubkey"):
            item["wallet_pubkey"] = row["wallet_pubkey"]
        if row.get("org_domain"):
            item["org_domain"] = row["org_domain"]
        items.append(item)
    return items
