import asyncio
import json
import os
import tempfile

from fastapi import APIRouter, File, HTTPException, UploadFile

from config import Settings
from routes.verify import run_verifier

router = APIRouter()


@router.post("/prove")
async def prove(file: UploadFile = File(...)):
    settings = Settings()
    file_bytes = await file.read()
    filename = file.filename or "upload"

    # 1. Verify
    verify_output = await run_verifier(file_bytes, filename, settings)

    # 2. Write file to temp (for prover)
    ext = os.path.splitext(filename)[1] if filename else ""
    media_tmp = tempfile.NamedTemporaryFile(suffix=ext, delete=False)
    media_tmp.write(file_bytes)
    media_tmp.close()

    # 3. Create sidecar temp file
    sidecar_tmp = tempfile.NamedTemporaryFile(suffix=".json", delete=False)
    sidecar_tmp.close()

    try:
        # 4. Run prover binary
        prover_bin = os.path.join(settings.prover_dir, "target/release/prove")
        args = [
            prover_bin,
            "--media", media_tmp.name,
            "--trust-dir", settings.trust_dir,
            "--json-out", sidecar_tmp.name,
        ]
        use_mock = settings.prover_mock != "false"
        if use_mock:
            args.append("--mock")

        proc = await asyncio.create_subprocess_exec(
            *args,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            cwd=settings.prover_dir,
        )
        stdout, stderr = await proc.communicate()

        if proc.returncode != 0:
            raise HTTPException(
                500,
                f"prover failed:\n--- stdout ---\n{stdout.decode()}\n--- stderr ---\n{stderr.decode()}",
            )

        # 5. Read sidecar JSON
        with open(sidecar_tmp.name) as f:
            sidecar = json.load(f)

        return {
            "proof": sidecar.get("proof", ""),
            "public_outputs": sidecar.get("public_values", ""),
            "verify_output": verify_output,
        }
    finally:
        os.unlink(media_tmp.name)
        os.unlink(sidecar_tmp.name)
