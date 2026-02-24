"""TLSH + MobileCLIP2-S0 similarity computation.

Call init_similarity() once at startup to load the CLIP model.
Then use compute_tlsh() / compute_clip_embedding() per-file.

Supports cross-modal embeddings:
  - Images: encode_image() via PIL
  - Videos: sample 8 frames via ffmpeg, encode_image() each, average pool
  - PDFs: extract text via PyMuPDF, encode_text()
  - Text files: decode UTF-8, encode_text()
"""

import logging
import os
import re
import subprocess
import tempfile
from io import BytesIO

import tlsh
import torch
import open_clip
from PIL import Image

log = logging.getLogger(__name__)

_model = None
_preprocess = None
_tokenizer = None
_device = "cpu"

# Content types that should use text extraction → encode_text()
_TEXT_CONTENT_TYPES = ("application/pdf", "text/")


def init_similarity():
    """Load MobileCLIP2-S0 model. Call once at startup."""
    global _model, _preprocess, _tokenizer, _device

    log.info("Loading MobileCLIP2-S0 model...")
    model, _, preprocess = open_clip.create_model_and_transforms(
        "MobileCLIP2-S0", pretrained="dfndr2b"
    )
    model.eval()

    # Reparameterize MobileOne BatchNorm layers for inference
    from timm.utils import reparameterize_model

    model = reparameterize_model(model)

    _model = model
    _preprocess = preprocess
    _tokenizer = open_clip.get_tokenizer("MobileCLIP2-S0")
    _device = "cpu"
    log.info("MobileCLIP2-S0 loaded (device=%s)", _device)


def compute_tlsh(file_bytes: bytes) -> str | None:
    """Compute TLSH hash from raw file bytes.

    Returns hex string (~70 chars) or None if file is too small/uniform.
    """
    h = tlsh.hash(file_bytes)
    return h if h else None


def compute_clip_embedding(
    file_bytes: bytes, content_type: str | None = None
) -> list[float] | None:
    """Compute CLIP embedding from file bytes.

    Waterfall:
      1. Try image encoding (PIL)
      2. If image fails and content_type is video/* → sample 8 frames, average
      3. If image fails and content_type is text-like → extract text, encode_text()
      4. Return None if nothing works

    Returns 512-dim L2-normalized float list, or None.
    """
    if _model is None or _preprocess is None:
        return None

    # 1. Try image encoding
    try:
        img = Image.open(BytesIO(file_bytes)).convert("RGB")
        tensor = _preprocess(img).unsqueeze(0).to(_device)
        with torch.no_grad():
            features = _model.encode_image(tensor)
            features /= features.norm(dim=-1, keepdim=True)
        return features[0].tolist()
    except Exception:
        pass

    ct = (content_type or "").lower()

    # 2. Try video frame sampling
    if ct.startswith("video/"):
        emb = _encode_video_frames(file_bytes)
        if emb is not None:
            return emb

    # 3. Try text extraction
    if any(ct.startswith(prefix) for prefix in _TEXT_CONTENT_TYPES):
        text = _extract_text(file_bytes, ct)
        if text:
            emb = _encode_text(text)
            if emb is not None:
                return emb

    log.debug("CLIP embedding skipped (unsupported file type: %s)", ct)
    return None


def _extract_text(file_bytes: bytes, content_type: str) -> str | None:
    """Extract text from a file based on its content type."""
    text = None

    if content_type.startswith("application/pdf"):
        try:
            import fitz

            doc = fitz.open(stream=file_bytes, filetype="pdf")
            text = " ".join(page.get_text() for page in doc)
            doc.close()
        except Exception:
            log.debug("PDF text extraction failed", exc_info=True)
            return None

    elif content_type.startswith("text/html"):
        try:
            raw = file_bytes.decode("utf-8", errors="ignore")
            # Strip HTML tags
            text = re.sub(r"<[^>]+>", " ", raw)
            text = re.sub(r"\s+", " ", text).strip()
        except Exception:
            return None

    elif content_type.startswith("text/"):
        # Plain text, CSV, markdown, etc.
        try:
            text = file_bytes.decode("utf-8", errors="ignore")
        except Exception:
            return None

    if text:
        # Truncate to ~1000 chars — CLIP tokenizer handles the rest (77 tokens max)
        return text[:1000].strip() or None
    return None


def _encode_text(text: str) -> list[float] | None:
    """Encode text string via CLIP text encoder. Returns 512-dim normalized list."""
    if _model is None or _tokenizer is None:
        return None
    try:
        tokens = _tokenizer([text]).to(_device)
        with torch.no_grad():
            features = _model.encode_text(tokens)
            features /= features.norm(dim=-1, keepdim=True)
        return features[0].tolist()
    except Exception:
        log.debug("Text encoding failed", exc_info=True)
        return None


def _encode_video_frames(file_bytes: bytes, num_frames: int = 8) -> list[float] | None:
    """Sample frames from a video via ffmpeg and average their CLIP embeddings."""
    if _model is None or _preprocess is None:
        return None

    tmp = tempfile.NamedTemporaryFile(suffix=".mp4", delete=False)
    try:
        tmp.write(file_bytes)
        tmp.close()

        # Get video duration
        duration = _get_video_duration(tmp.name)
        if duration is None or duration <= 0:
            return None

        # Sample timestamps evenly (including t=0)
        if duration < 1:
            timestamps = [0.0]
        else:
            timestamps = [duration * i / (num_frames - 1) for i in range(num_frames)]

        # Extract and encode each frame
        embeddings = []
        for ts in timestamps:
            frame_bytes = _extract_frame_at(tmp.name, ts)
            if frame_bytes is None:
                continue
            try:
                img = Image.open(BytesIO(frame_bytes)).convert("RGB")
                tensor = _preprocess(img).unsqueeze(0).to(_device)
                with torch.no_grad():
                    features = _model.encode_image(tensor)
                    features /= features.norm(dim=-1, keepdim=True)
                embeddings.append(features[0])
            except Exception:
                continue

        if not embeddings:
            return None

        # Average pool and re-normalize
        stacked = torch.stack(embeddings)
        avg = stacked.mean(dim=0)
        avg /= avg.norm()
        return avg.tolist()

    except Exception:
        log.debug("Video frame encoding failed", exc_info=True)
        return None
    finally:
        os.unlink(tmp.name)


def _get_video_duration(path: str) -> float | None:
    """Get video duration in seconds using ffprobe."""
    try:
        result = subprocess.run(
            [
                "ffprobe",
                "-v", "quiet",
                "-print_format", "json",
                "-show_format",
                path,
            ],
            capture_output=True,
            timeout=10,
        )
        if result.returncode != 0:
            return None
        import json

        info = json.loads(result.stdout)
        return float(info["format"]["duration"])
    except Exception:
        return None


def _extract_frame_at(video_path: str, timestamp: float) -> bytes | None:
    """Extract a single frame at the given timestamp as PNG bytes."""
    try:
        result = subprocess.run(
            [
                "ffmpeg",
                "-ss", str(timestamp),
                "-i", video_path,
                "-frames:v", "1",
                "-f", "image2pipe",
                "-vcodec", "png",
                "-",
            ],
            capture_output=True,
            timeout=10,
        )
        if result.returncode != 0 or not result.stdout:
            return None
        return result.stdout
    except Exception:
        return None


def tlsh_distance(h1: str, h2: str) -> int:
    """Compute TLSH distance between two hashes.

    Returns 0 for identical, <100 for near-duplicate, >300 for unrelated.
    """
    return tlsh.diff(h1, h2)
