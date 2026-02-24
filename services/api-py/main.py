import os
import threading

from dotenv import load_dotenv

# Load .env before anything reads env vars
load_dotenv(os.path.join(os.path.dirname(__file__), "../../.env"))

from pathlib import Path

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import FileResponse
from fastapi.staticfiles import StaticFiles

from config import Settings
from routes import verify, attest, prove, submit, attestation, edge, query, similar, org, did_route, auth_routes
import db
from similarity import init_similarity

settings = Settings()
app = FastAPI()


@app.on_event("startup")
async def startup():
    await db.init_db(settings.database_url)
    # Load CLIP model in background so health checks pass immediately
    threading.Thread(target=init_similarity, daemon=True).start()


@app.on_event("shutdown")
async def shutdown():
    await db.close_db()

# CORS — allow all (matches Rust API)
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_methods=["*"],
    allow_headers=["*"],
)


@app.get("/api/health")
async def health():
    return "ok"


# API routes
app.include_router(verify.router, prefix="/api")
app.include_router(attest.router, prefix="/api")
app.include_router(prove.router, prefix="/api")
app.include_router(submit.router, prefix="/api")
app.include_router(attestation.router, prefix="/api")
app.include_router(auth_routes.router, prefix="/api/auth")
app.include_router(edge.router, prefix="/api/edge")
app.include_router(query.router, prefix="/api")
app.include_router(similar.router, prefix="/api/v1/similar")
app.include_router(org.router, prefix="/api/org")
app.include_router(did_route.router, prefix="/api")

# .well-known DID document (must be before SPA fallback)
from routes.did_route import platform_did
app.add_api_route("/.well-known/did.json", platform_did, methods=["GET"])

# Static files — Vue SPA (must be last)
static_dir = settings.static_dir
if os.path.isdir(static_dir):
    index_html = Path(static_dir) / "index.html"

    @app.get("/{full_path:path}")
    async def spa_fallback(full_path: str):
        # Serve actual static files if they exist, otherwise index.html for SPA routing
        static_path = Path(static_dir) / full_path
        if static_path.is_file():
            return FileResponse(static_path)
        return FileResponse(index_html)

