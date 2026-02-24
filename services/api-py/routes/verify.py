import asyncio
import hashlib
import json
import os
import tempfile

from fastapi import APIRouter, File, HTTPException, UploadFile

from config import Settings

router = APIRouter()

MAX_FILE_SIZE = 50 * 1024 * 1024  # 50 MB
ALLOWED_MIME_PREFIXES = ("image/", "video/", "audio/", "application/pdf", "text/")
VERIFIER_TIMEOUT = 30  # seconds


def validate_upload(file_bytes: bytes, content_type: str | None = None):
    if len(file_bytes) > MAX_FILE_SIZE:
        raise HTTPException(413, f"file too large: {len(file_bytes)} bytes (max {MAX_FILE_SIZE})")
    if content_type:
        ct = content_type.lower()
        if not any(ct.startswith(p) for p in ALLOWED_MIME_PREFIXES):
            raise HTTPException(415, f"unsupported media type: {ct}")


async def run_verifier(file_bytes: bytes, filename: str, settings: Settings) -> dict:
    ext = os.path.splitext(filename)[1] if filename else ""
    tmp = tempfile.NamedTemporaryFile(suffix=ext, delete=False)
    try:
        tmp.write(file_bytes)
        tmp.close()

        proc = await asyncio.create_subprocess_exec(
            settings.verifier_bin, tmp.name,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            env={**os.environ, "TRUST_DIR": settings.trust_dir},
        )
        try:
            stdout, stderr = await asyncio.wait_for(proc.communicate(), timeout=VERIFIER_TIMEOUT)
        except asyncio.TimeoutError:
            proc.kill()
            raise HTTPException(504, "verifier timed out")

        if proc.returncode != 0:
            # Verifier can't handle this file type (e.g. PDF without C2PA support).
            # Return an unsigned result with the content hash computed in Python.
            content_hash = hashlib.sha256(file_bytes).hexdigest()
            return {
                "path": filename,
                "content_hash": content_hash,
                "has_c2pa": False,
                "trust_list_match": None,
                "validation_state": None,
                "validation_error_count": None,
                "validation_codes": None,
                "title": None,
                "format": None,
                "digital_source_type": None,
                "claim_generator": None,
                "software_agent": None,
                "issuer": None,
                "common_name": None,
                "signing_time": None,
                "sig_algorithm": None,
                "actions": None,
                "ingredients": None,
                "manifest_store": None,
                "error": None,
            }

        return json.loads(stdout.decode())
    finally:
        os.unlink(tmp.name)


@router.post("/verify")
async def verify(file: UploadFile = File(...)):
    settings = Settings()
    file_bytes = await file.read()
    validate_upload(file_bytes, file.content_type)
    result = await run_verifier(file_bytes, file.filename or "upload", settings)
    return result
