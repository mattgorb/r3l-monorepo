import hashlib

from fastapi import APIRouter, File, HTTPException, UploadFile

from routes.verify import validate_upload
from similarity import compute_tlsh, compute_clip_embedding, tlsh_distance
import db

router = APIRouter()

MAX_RESULTS = 20


def _classify_match(
    tlsh_dist: int | None, clip_sim: float | None
) -> str:
    """Classify a match as exact, near_duplicate, visual_match, or unrelated."""
    if tlsh_dist is not None and tlsh_dist == 0 and (clip_sim is None or clip_sim >= 0.95):
        return "exact"
    if tlsh_dist is not None and tlsh_dist <= 100 and (clip_sim is None or clip_sim >= 0.85):
        return "near_duplicate"
    if clip_sim is not None and clip_sim >= 0.6:
        return "visual_match"
    return "unrelated"


def _build_match(row: dict, tlsh_dist: int | None, clip_sim: float | None) -> dict:
    match_type = _classify_match(tlsh_dist, clip_sim)
    return {
        "content_hash": row["content_hash"],
        "match_type": match_type,
        "tlsh_hash": row.get("tlsh_hash"),
        "tlsh_distance": tlsh_dist,
        "clip_similarity": round(float(clip_sim), 4) if clip_sim is not None else None,
        "issuer": row.get("issuer"),
        "trust_list_match": row.get("trust_list_match"),
        "has_c2pa": row.get("has_c2pa"),
        "timestamp": row.get("created_at"),
    }


def _sort_matches(matches: list[dict]) -> list[dict]:
    """Sort: exact first, then near_duplicate, visual_match, unrelated.
    Within each group, sort by best similarity (highest clip, lowest tlsh)."""
    type_order = {"exact": 0, "near_duplicate": 1, "visual_match": 2, "unrelated": 3}
    matches.sort(key=lambda m: (
        type_order.get(m["match_type"], 9),
        -(m["clip_similarity"] or 0),
        (m["tlsh_distance"] if m["tlsh_distance"] is not None else 9999),
    ))
    return matches[:MAX_RESULTS]


@router.post("")
async def search_similar_by_file(file: UploadFile = File(...)):
    """Upload a file and find similar attested content."""
    file_bytes = await file.read()
    validate_upload(file_bytes, file.content_type)

    # Compute hashes
    content_hash = hashlib.sha256(file_bytes).hexdigest()
    query_tlsh = compute_tlsh(file_bytes)
    query_clip = compute_clip_embedding(file_bytes, file.content_type)

    matches = []
    seen_hashes = set()

    # 1. Exact match
    exact = await db.get_attestation(content_hash)
    if exact:
        matches.append({
            "content_hash": exact["content_hash"],
            "match_type": "exact",
            "tlsh_hash": exact.get("tlsh_hash"),
            "tlsh_distance": 0,
            "clip_similarity": 1.0,
            "issuer": exact.get("issuer"),
            "trust_list_match": exact.get("trust_list_match"),
            "has_c2pa": exact.get("has_c2pa"),
            "timestamp": exact.get("created_at"),
        })
        seen_hashes.add(content_hash)

    # 2. TLSH scan — compare against all attestations with TLSH hashes (no distance cutoff)
    if query_tlsh:
        tlsh_rows = await db.get_all_with_tlsh()
        for row in tlsh_rows:
            if row["content_hash"] in seen_hashes:
                continue
            dist = tlsh_distance(query_tlsh, row["tlsh_hash"])
            clip_sim = None
            if query_clip and row.get("clip_embedding"):
                a, b = query_clip, list(row["clip_embedding"])
                dot = sum(x * y for x, y in zip(a, b))
                clip_sim = dot  # both are L2-normalized, so dot = cosine
            seen_hashes.add(row["content_hash"])
            matches.append(_build_match(row, dist, clip_sim))

    # 3. CLIP embedding search (pgvector nearest neighbors)
    if query_clip:
        clip_rows = await db.search_similar_clip(query_clip, limit=MAX_RESULTS)
        for row in clip_rows:
            if row["content_hash"] in seen_hashes:
                continue
            clip_sim = row.get("clip_similarity")
            tlsh_dist = None
            if query_tlsh and row.get("tlsh_hash"):
                tlsh_dist = tlsh_distance(query_tlsh, row["tlsh_hash"])
            seen_hashes.add(row["content_hash"])
            matches.append(_build_match(row, tlsh_dist, clip_sim))

    return {"query_hash": content_hash, "query_tlsh": query_tlsh, "matches": _sort_matches(matches)}


@router.get("/{content_hash}")
async def search_similar_by_hash(content_hash: str):
    """Find content similar to an existing attestation."""
    existing = await db.get_attestation(content_hash)
    if not existing:
        raise HTTPException(404, "attestation not found")

    query_tlsh = existing.get("tlsh_hash")
    query_clip = existing.get("clip_embedding")
    if query_clip is not None:
        query_clip = list(query_clip)

    matches = []
    seen_hashes = {content_hash}

    # 1. TLSH scan (no distance cutoff)
    if query_tlsh:
        tlsh_rows = await db.get_all_with_tlsh()
        for row in tlsh_rows:
            if row["content_hash"] in seen_hashes:
                continue
            dist = tlsh_distance(query_tlsh, row["tlsh_hash"])
            clip_sim = None
            if query_clip and row.get("clip_embedding"):
                a, b = query_clip, list(row["clip_embedding"])
                dot = sum(x * y for x, y in zip(a, b))
                clip_sim = dot
            seen_hashes.add(row["content_hash"])
            matches.append(_build_match(row, dist, clip_sim))

    # 2. CLIP embedding search
    if query_clip:
        clip_rows = await db.search_similar_clip(query_clip, limit=MAX_RESULTS)
        for row in clip_rows:
            if row["content_hash"] in seen_hashes:
                continue
            clip_sim = row.get("clip_similarity")
            tlsh_dist = None
            if query_tlsh and row.get("tlsh_hash"):
                tlsh_dist = tlsh_distance(query_tlsh, row["tlsh_hash"])
            seen_hashes.add(row["content_hash"])
            matches.append(_build_match(row, tlsh_dist, clip_sim))

    return {"query_hash": content_hash, "query_tlsh": query_tlsh, "matches": _sort_matches(matches)}
