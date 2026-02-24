from fastapi import APIRouter, HTTPException
from urllib.parse import unquote

from did import resolve_did
from config import Settings

router = APIRouter()


@router.get("/did/{did:path}")
async def resolve(did: str):
    """Resolve any DID string to a W3C DID Document."""
    did = unquote(did)
    if not did.startswith("did:"):
        raise HTTPException(400, "invalid DID — must start with 'did:'")
    doc = resolve_did(did)
    if "error" in doc:
        raise HTTPException(400, doc["error"])
    return doc


@router.get("/.well-known/did.json")
async def platform_did():
    """Serve the DID document for the R3L platform itself."""
    settings = Settings()
    # Extract domain from public_url
    domain = settings.public_url.replace("https://", "").replace("http://", "").split("/")[0]
    return {
        "@context": ["https://www.w3.org/ns/did/v1"],
        "id": f"did:web:{domain}",
        "verificationMethod": [],
        "service": [
            {
                "id": f"did:web:{domain}#attestation",
                "type": "R3LAttestation",
                "serviceEndpoint": settings.public_url + "/api",
            },
            {
                "id": f"did:web:{domain}#verify",
                "type": "R3LVerification",
                "serviceEndpoint": settings.public_url + "/api/verify",
            },
        ],
    }
