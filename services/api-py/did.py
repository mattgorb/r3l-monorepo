"""DID (Decentralized Identifier) formatting utilities.

Supports did:pkh, did:jwk, did:key, and did:web methods.
"""

import base64
import json

import base58


def format_did_pkh(wallet_pubkey: str) -> str:
    """did:pkh:solana:{pubkey} — CAIP-10 style for Solana wallets."""
    return f"did:pkh:solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp:{wallet_pubkey}"


def format_did_jwk(ed25519_pubkey_bytes: bytes) -> str:
    """did:jwk:{base64url} — RFC 8037 OKP/Ed25519 JWK."""
    jwk = {
        "kty": "OKP",
        "crv": "Ed25519",
        "x": base64.urlsafe_b64encode(ed25519_pubkey_bytes).rstrip(b"=").decode(),
    }
    jwk_json = json.dumps(jwk, separators=(",", ":"), sort_keys=True)
    encoded = base64.urlsafe_b64encode(jwk_json.encode()).rstrip(b"=").decode()
    return f"did:jwk:{encoded}"


def format_did_key(ed25519_pubkey_bytes: bytes) -> str:
    """did:key:z{multibase} — multicodec 0xed01 prefix for Ed25519."""
    # Ed25519 multicodec prefix: 0xed (varint encoded as 0xed 0x01)
    prefixed = b"\xed\x01" + ed25519_pubkey_bytes
    multibase = "z" + base58.b58encode(prefixed).decode()
    return f"did:key:{multibase}"


def format_did_web(domain: str) -> str:
    """did:web:{domain} — domain-based DID."""
    return f"did:web:{domain}"


def resolve_did(did_string: str) -> dict:
    """Resolve a DID string to a W3C DID Document."""
    parts = did_string.split(":")
    if len(parts) < 3 or parts[0] != "did":
        return {"error": "invalid DID format"}

    method = parts[1]
    doc = {
        "@context": ["https://www.w3.org/ns/did/v1"],
        "id": did_string,
    }

    if method == "web":
        domain = parts[2]
        doc["service"] = [{
            "id": f"{did_string}#r3l",
            "type": "R3LAttestation",
            "serviceEndpoint": f"https://{domain}",
        }]
    elif method == "pkh":
        # did:pkh:solana:chainId:address
        if len(parts) >= 5:
            doc["verificationMethod"] = [{
                "id": f"{did_string}#key-0",
                "type": "Ed25519VerificationKey2020",
                "controller": did_string,
                "blockchainAccountId": f"solana:{parts[3]}:{parts[4]}",
            }]
    elif method == "jwk":
        encoded = parts[2]
        # Restore base64url padding
        padded = encoded + "=" * (4 - len(encoded) % 4) if len(encoded) % 4 else encoded
        try:
            jwk = json.loads(base64.urlsafe_b64decode(padded))
            doc["verificationMethod"] = [{
                "id": f"{did_string}#0",
                "type": "JsonWebKey2020",
                "controller": did_string,
                "publicKeyJwk": jwk,
            }]
        except Exception:
            pass
    elif method == "key":
        multibase = parts[2]
        if multibase.startswith("z"):
            try:
                decoded = base58.b58decode(multibase[1:])
                # Strip 0xed01 prefix
                if decoded[:2] == b"\xed\x01":
                    pub_bytes = decoded[2:]
                    doc["verificationMethod"] = [{
                        "id": f"{did_string}#{multibase}",
                        "type": "Ed25519VerificationKey2020",
                        "controller": did_string,
                        "publicKeyMultibase": multibase,
                    }]
            except Exception:
                pass

    return doc


def get_all_dids_for_org(org: dict, wallet_pubkey: str | None = None) -> dict:
    """Return all DID formats for an organization."""
    dids = {}
    domain = org.get("domain")
    if domain:
        dids["did:web"] = format_did_web(domain)

    if wallet_pubkey:
        dids["did:pkh"] = format_did_pkh(wallet_pubkey)
        try:
            pub_bytes = base58.b58decode(wallet_pubkey)
            dids["did:jwk"] = format_did_jwk(pub_bytes)
            dids["did:key"] = format_did_key(pub_bytes)
        except Exception:
            pass

    return dids
