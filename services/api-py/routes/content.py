from fastapi import APIRouter, HTTPException
from fastapi.responses import Response

from storage import get_storage
import db

router = APIRouter()


@router.get("/content/{content_hash}")
async def get_content(content_hash: str):
    """Retrieve stored content by its SHA-256 hash."""
    att = await db.get_attestation(content_hash)
    if not att:
        raise HTTPException(404, "attestation not found")
    if not att.get("stored"):
        raise HTTPException(404, "content not stored for this attestation")

    storage = get_storage()
    result = await storage.get(content_hash)
    if result is None:
        raise HTTPException(404, "content not found in storage")

    data, content_type = result
    return Response(content=data, media_type=content_type)
