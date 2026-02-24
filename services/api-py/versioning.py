import hashlib
import os

VERIFIER_VERSION = "0.1.0"


def compute_trust_bundle_hash(trust_dir: str) -> str:
    """SHA-256 of sorted, concatenated PEM files from official/ and curated/ subdirs."""
    hasher = hashlib.sha256()
    for subdir in ["official", "curated"]:
        dirpath = os.path.join(trust_dir, subdir)
        if not os.path.isdir(dirpath):
            continue
        pem_files = sorted(f for f in os.listdir(dirpath) if f.endswith(".pem"))
        for fname in pem_files:
            with open(os.path.join(dirpath, fname), "rb") as f:
                hasher.update(f.read())
    return hasher.hexdigest()
