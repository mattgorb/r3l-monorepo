import struct

from solders.pubkey import Pubkey
from solana.rpc.api import Client as SolanaClient

from solana_tx import ATTESTATION_SEED, find_pda

# ── Account discriminator ──────────────────────────────────────────
ATTESTATION_DISC = bytes([152, 125, 183, 86, 36, 146, 121, 73])

ZERO_PUBKEY = b"\x00" * 32


def _read_borsh_string(data: bytes, off: int) -> tuple[str, int]:
    length = struct.unpack_from("<I", data, off)[0]
    off += 4
    s = data[off:off + length].decode("utf-8")
    return s, off + length


def deserialize_attestation(data: bytes) -> dict | None:
    if len(data) < 8 or data[:8] != ATTESTATION_DISC:
        return None

    try:
        off = 8
        content_hash = data[off:off + 32]; off += 32
        has_c2pa = bool(data[off]); off += 1

        # 8 C2PA string fields
        strings = []
        for _ in range(8):
            s, off = _read_borsh_string(data, off)
            strings.append(s)

        submitted_by = data[off:off + 32]; off += 32
        timestamp = struct.unpack_from("<q", data, off)[0]; off += 8
        _bump = data[off]; off += 1
        proof_type, off = _read_borsh_string(data, off)

        # New fields (may not exist in old accounts)
        email_domain, off = _read_borsh_string(data, off)
        email_hash = data[off:off + 32]; off += 32
        wallet = data[off:off + 32]; off += 32
        wallet_sig = data[off:off + 64]; off += 64
        verifier_version, off = _read_borsh_string(data, off)
        trust_bundle_hash, off = _read_borsh_string(data, off)

        result = {
            "content_hash": content_hash.hex(),
            "has_c2pa": has_c2pa,
            "trust_list_match": strings[0],
            "validation_state": strings[1],
            "digital_source_type": strings[2],
            "issuer": strings[3],
            "common_name": strings[4],
            "software_agent": strings[5],
            "signing_time": strings[6],
            "cert_fingerprint": strings[7],
            "submitted_by": str(Pubkey.from_bytes(submitted_by)),
            "timestamp": timestamp,
            "proof_type": proof_type,
        }

        if email_domain:
            result["email_domain"] = email_domain
        if wallet != ZERO_PUBKEY:
            result["wallet_pubkey"] = str(Pubkey.from_bytes(wallet))
        if wallet_sig != b"\x00" * 64:
            result["wallet_sig"] = wallet_sig.hex()
        if verifier_version:
            result["verifier_version"] = verifier_version
        if trust_bundle_hash:
            result["trust_bundle_hash"] = trust_bundle_hash

        return result
    except Exception:
        return None


def lookup_attestation(rpc_url: str, program_id_str: str, content_hash_hex: str) -> dict | None:
    try:
        content_hash_bytes = bytes.fromhex(content_hash_hex)
    except ValueError:
        return None
    if len(content_hash_bytes) != 32:
        return None

    program_id = Pubkey.from_string(program_id_str)
    pda, _ = find_pda([ATTESTATION_SEED, content_hash_bytes], program_id)

    client = SolanaClient(rpc_url)
    resp = client.get_account_info(pda)
    if resp.value is None:
        return None

    data = resp.value.data
    return deserialize_attestation(data)


def list_all_attestations(rpc_url: str, program_id_str: str) -> list[dict]:
    program_id = Pubkey.from_string(program_id_str)
    client = SolanaClient(rpc_url)
    items = []

    try:
        resp = client.get_program_accounts(program_id)
        for keyed in resp.value:
            data = keyed.account.data
            if len(data) < 8:
                continue

            att = deserialize_attestation(data)
            if att:
                item = {
                    "content_hash": att["content_hash"],
                    "proof_type": att["proof_type"],
                    "timestamp": att["timestamp"],
                }
                if att.get("issuer"):
                    item["issuer"] = att["issuer"]
                if att.get("trust_list_match"):
                    item["trust_list_match"] = att["trust_list_match"]
                if att.get("email_domain"):
                    item["email_domain"] = att["email_domain"]
                if att.get("wallet_pubkey"):
                    item["wallet_pubkey"] = att["wallet_pubkey"]
                items.append(item)
    except Exception:
        pass

    items.sort(key=lambda x: x["timestamp"], reverse=True)
    return items
