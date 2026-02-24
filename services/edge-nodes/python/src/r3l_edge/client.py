"""R3L Edge SDK — programmatic client for Python integrations."""

import json
import logging
import os
import subprocess
import urllib.request
import urllib.error
from io import BytesIO

import nacl.signing
import base58

log = logging.getLogger(__name__)

# Lazy-loaded similarity modules
_clip_model = None
_clip_preprocess = None
_similarity_initialized = False


def _init_similarity():
    """Lazy-load TLSH and MobileCLIP2-S0 for similarity computation."""
    global _clip_model, _clip_preprocess, _similarity_initialized
    if _similarity_initialized:
        return
    _similarity_initialized = True
    try:
        import torch
        import open_clip
        from timm.utils import reparameterize_model

        model, _, preprocess = open_clip.create_model_and_transforms(
            "MobileCLIP2-S0", pretrained="dfndr2b"
        )
        model.eval()
        model = reparameterize_model(model)
        _clip_model = model
        _clip_preprocess = preprocess
        log.info("MobileCLIP2-S0 loaded for edge similarity")
    except Exception:
        log.warning("Could not load MobileCLIP2-S0 — CLIP embeddings disabled", exc_info=True)


def compute_tlsh_hash(file_bytes: bytes) -> str:
    """Compute TLSH hash from file bytes. Returns hex string or empty."""
    try:
        import tlsh
        h = tlsh.hash(file_bytes)
        return h if h else ""
    except Exception:
        return ""


def compute_clip_embedding(file_bytes: bytes) -> list[float]:
    """Compute MobileCLIP2-S0 image embedding. Returns 512-dim list or empty."""
    _init_similarity()
    if _clip_model is None or _clip_preprocess is None:
        return []
    try:
        import torch
        from PIL import Image

        img = Image.open(BytesIO(file_bytes)).convert("RGB")
        tensor = _clip_preprocess(img).unsqueeze(0)
        with torch.no_grad():
            features = _clip_model.encode_image(tensor)
            features /= features.norm(dim=-1, keepdim=True)
        return features[0].tolist()
    except Exception:
        log.warning("Failed to compute CLIP embedding", exc_info=True)
        return []


class R3LEdgeClient:
    """Client for interacting with the R3L API from edge nodes."""

    def __init__(
        self,
        api_url: str = "http://localhost:3001",
        api_key: str = "",
        keypair_path: str | None = None,
    ):
        self.api_url = api_url.rstrip("/")
        self.api_key = api_key
        self._signing_key: nacl.signing.SigningKey | None = None

        if keypair_path and os.path.exists(keypair_path):
            self.load_keypair(keypair_path)

    # ── Keypair management ────────────────────────────────────────

    def load_keypair(self, path: str):
        """Load an Ed25519 keypair from a Solana-style JSON array file."""
        with open(path) as f:
            raw = json.load(f)
        self._signing_key = nacl.signing.SigningKey(bytes(raw[:32]))

    def generate_keypair(self, path: str):
        """Generate a new Ed25519 keypair and save to file."""
        self._signing_key = nacl.signing.SigningKey.generate()
        full = list(self._signing_key.encode() + self._signing_key.verify_key.encode())
        with open(path, "w") as f:
            json.dump(full, f)

    @property
    def pubkey(self) -> str | None:
        """Base58-encoded public key, or None if no keypair loaded."""
        if self._signing_key is None:
            return None
        return base58.b58encode(self._signing_key.verify_key.encode()).decode()

    def sign(self, message: str) -> str:
        """Sign a message and return base58-encoded signature."""
        if self._signing_key is None:
            raise RuntimeError("No keypair loaded")
        sig = self._signing_key.sign(message.encode()).signature
        return base58.b58encode(sig).decode()

    # ── API calls ─────────────────────────────────────────────────

    def register(self, name: str | None = None) -> dict:
        """Register this edge node. Returns dict with api_key, pubkey, name."""
        if self._signing_key is None:
            raise RuntimeError("No keypair loaded — call load_keypair() or generate_keypair() first")

        pubkey = self.pubkey
        sig = self.sign("R3L: register")

        resp = self._post("/api/edge/register", {
            "pubkey": pubkey,
            "message": "R3L: register",
            "signature": sig,
            "name": name or f"edge-{pubkey[:8]}",
        })
        self.api_key = resp["api_key"]
        return resp

    def attest(
        self,
        content_hash: str,
        has_c2pa: bool = False,
        trust_list_match: str = "",
        validation_state: str = "",
        digital_source_type: str = "",
        issuer: str = "",
        common_name: str = "",
        software_agent: str = "",
        signing_time: str = "",
        tlsh_hash: str = "",
        clip_embedding: list[float] | None = None,
    ) -> dict:
        """Submit an attestation for a content hash. Returns tx details."""
        if not self.api_key:
            raise RuntimeError("No API key — call register() first")

        body: dict = {
            "content_hash": content_hash,
            "has_c2pa": has_c2pa,
            "trust_list_match": trust_list_match,
            "validation_state": validation_state,
            "digital_source_type": digital_source_type,
            "issuer": issuer,
            "common_name": common_name,
            "software_agent": software_agent,
            "signing_time": signing_time,
        }

        if tlsh_hash:
            body["tlsh_hash"] = tlsh_hash
        if clip_embedding:
            body["clip_embedding"] = clip_embedding

        if self._signing_key:
            wallet_sig = self.sign(f"R3L: attest {content_hash}")
            body["wallet_signature"] = wallet_sig

        return self._post("/api/edge/attest", body, {"X-API-Key": self.api_key})

    def verify_and_attest(self, file_path: str, verifier_bin: str = "verifier", trust_dir: str = "") -> dict:
        """Run the verifier binary on a file, then submit the attestation."""
        cmd = [verifier_bin, file_path]
        env = None
        if trust_dir:
            env = {**os.environ, "TRUST_DIR": trust_dir}

        result = subprocess.run(cmd, capture_output=True, text=True, timeout=30, env=env)

        # Verifier outputs JSON to stdout even on errors (with an "error" field)
        # Only treat as fatal if we can't parse stdout at all
        if not result.stdout.strip():
            raise RuntimeError(f"Verifier produced no output: {result.stderr}")

        output = json.loads(result.stdout)
        if output.get("error"):
            raise RuntimeError(f"Verifier error: {output['error']}")

        # Compute similarity hashes from file bytes
        file_bytes = open(file_path, "rb").read()
        tlsh_hash = compute_tlsh_hash(file_bytes)
        clip_emb = compute_clip_embedding(file_bytes)

        return self.attest(
            content_hash=output["content_hash"],
            has_c2pa=output.get("has_c2pa", False),
            trust_list_match=output.get("trust_list_match") or "",
            validation_state=output.get("validation_state") or "",
            digital_source_type=output.get("digital_source_type") or "",
            issuer=output.get("issuer") or "",
            common_name=output.get("common_name") or "",
            software_agent=output.get("software_agent") or "",
            signing_time=output.get("signing_time") or "",
            tlsh_hash=tlsh_hash,
            clip_embedding=clip_emb if clip_emb else None,
        )

    def query(self, content_hash: str) -> dict:
        """Query the structured trust verdict for a content hash."""
        return self._get(f"/api/v1/query/{content_hash}")

    def lookup(self, content_hash: str) -> dict:
        """Look up raw attestation data for a content hash."""
        return self._get(f"/api/attestation/{content_hash}")

    # ── HTTP helpers ──────────────────────────────────────────────

    def _post(self, path: str, body: dict, extra_headers: dict | None = None) -> dict:
        data = json.dumps(body).encode()
        headers = {"Content-Type": "application/json"}
        if extra_headers:
            headers.update(extra_headers)
        req = urllib.request.Request(f"{self.api_url}{path}", data=data, headers=headers, method="POST")
        with urllib.request.urlopen(req, timeout=30) as resp:
            return json.loads(resp.read())

    def _get(self, path: str) -> dict:
        req = urllib.request.Request(f"{self.api_url}{path}")
        with urllib.request.urlopen(req, timeout=30) as resp:
            return json.loads(resp.read())
