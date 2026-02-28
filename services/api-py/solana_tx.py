import json
import struct

from solders.compute_budget import set_compute_unit_limit
from solders.hash import Hash
from solders.instruction import AccountMeta, Instruction
from solders.keypair import Keypair
from solders.message import Message
from solders.pubkey import Pubkey
from solders.transaction import Transaction
from solana.rpc.api import Client as SolanaClient
from solana.rpc.commitment import Confirmed

SYSTEM_PROGRAM_ID = Pubkey.from_string("11111111111111111111111111111111")
ED25519_PROGRAM_ID = Pubkey.from_string("Ed25519SigVerify111111111111111111111111111")
INSTRUCTIONS_SYSVAR_ID = Pubkey.from_string("Sysvar1nstructions1111111111111111111111111")

# ── Instruction discriminators ──────────────────────────────────────
SUBMIT_ATTESTATION_DISC = bytes([238, 220, 255, 105, 183, 211, 40, 83])
SUBMIT_PROOF_DISC = bytes([54, 241, 46, 84, 4, 212, 46, 94])

# ── PDA seeds ───────────────────────────────────────────────────────
ATTESTATION_SEED = b"attestation"


def borsh_string(s: str) -> bytes:
    encoded = s.encode("utf-8")
    return struct.pack("<I", len(encoded)) + encoded


def borsh_vec(data: bytes) -> bytes:
    return struct.pack("<I", len(data)) + data


def load_keypair(path: str) -> Keypair:
    with open(path) as f:
        secret = json.load(f)
    return Keypair.from_bytes(bytes(secret))


def find_pda(seeds: list[bytes], program_id: Pubkey) -> tuple[Pubkey, int]:
    return Pubkey.find_program_address(seeds, program_id)


# ── Ed25519 precompile instruction ─────────────────────────────────

def create_ed25519_instruction(
    pubkey: bytes,
    signature: bytes,
    message: bytes,
) -> Instruction:
    """Build an Ed25519SigVerify precompile instruction (1 signature).

    Layout:
      [0..2]:    num_signatures (u8=1) + padding (u8=0)
      [2..16]:   Ed25519SignatureOffsets (7 x u16 LE)
      [16..80]:  64-byte signature
      [80..112]: 32-byte pubkey
      [112..]:   message bytes
    """
    assert len(pubkey) == 32
    assert len(signature) == 64

    sig_offset = 16          # where signature starts in instruction data
    sig_ix_index = 0xFFFF    # same instruction (not external)
    pubkey_offset = 80       # where pubkey starts
    pubkey_ix_index = 0xFFFF
    msg_offset = 112         # where message starts
    msg_size = len(message)
    msg_ix_index = 0xFFFF

    data = bytearray()
    # header
    data += struct.pack("<BB", 1, 0)  # num_signatures=1, padding=0
    # Ed25519SignatureOffsets
    data += struct.pack("<HHHHHHH",
        sig_offset, sig_ix_index,
        pubkey_offset, pubkey_ix_index,
        msg_offset, msg_size, msg_ix_index,
    )
    # signature + pubkey + message
    data += signature
    data += pubkey
    data += message

    return Instruction(ED25519_PROGRAM_ID, bytes(data), [])


# ── Instruction data encoders ───────────────────────────────────────

def encode_attestation_data(
    content_hash: bytes,
    has_c2pa: bool,
    trust_list_match: str,
    validation_state: str,
    digital_source_type: str,
    issuer: str,
    common_name: str,
    software_agent: str,
    signing_time: str,
    cert_fingerprint: str,
    email_domain: str = "",
    email_hash: bytes = b"\x00" * 32,
    wallet: bytes = b"\x00" * 32,
    verifier_version: str = "",
    trust_bundle_hash: str = "",
) -> bytes:
    data = bytearray(SUBMIT_ATTESTATION_DISC)
    data += content_hash
    data += struct.pack("?", has_c2pa)
    for s in [
        trust_list_match, validation_state, digital_source_type,
        issuer, common_name, software_agent, signing_time, cert_fingerprint,
        email_domain,
    ]:
        data += borsh_string(s)
    data += email_hash
    data += wallet
    data += borsh_string(verifier_version)
    data += borsh_string(trust_bundle_hash)
    return bytes(data)


def encode_proof_data(
    proof_bytes: bytes,
    public_inputs_bytes: bytes,
    content_hash: bytes,
    email_domain: str = "",
    email_hash: bytes = b"\x00" * 32,
    wallet: bytes = b"\x00" * 32,
    verifier_version: str = "",
    trust_bundle_hash: str = "",
) -> bytes:
    data = bytearray(SUBMIT_PROOF_DISC)
    data += borsh_vec(proof_bytes)
    data += borsh_vec(public_inputs_bytes)
    data += content_hash
    data += borsh_string(email_domain)
    data += email_hash
    data += wallet
    data += borsh_string(verifier_version)
    data += borsh_string(trust_bundle_hash)
    return bytes(data)


# ── Transaction builder ─────────────────────────────────────────────

def build_and_send_tx(
    rpc_url: str,
    keypair_path: str,
    program_id_str: str,
    ix_data: bytes,
    pda: Pubkey,
    compute_units: int = 200_000,
    extra_ixs: list[Instruction] | None = None,
) -> tuple[str, str]:
    """Build, sign, and send a Solana transaction. Returns (signature, pda_str)."""
    client = SolanaClient(rpc_url)
    payer = load_keypair(keypair_path)
    program_id = Pubkey.from_string(program_id_str)

    accounts = [
        AccountMeta(pda, is_signer=False, is_writable=True),
        AccountMeta(payer.pubkey(), is_signer=True, is_writable=True),
        AccountMeta(SYSTEM_PROGRAM_ID, is_signer=False, is_writable=False),
        AccountMeta(INSTRUCTIONS_SYSVAR_ID, is_signer=False, is_writable=False),
    ]

    ix = Instruction(program_id, ix_data, accounts)
    compute_ix = set_compute_unit_limit(compute_units)

    # Order: compute budget → extra instructions (Ed25519) → program instruction
    all_ixs = [compute_ix]
    if extra_ixs:
        all_ixs.extend(extra_ixs)
    all_ixs.append(ix)

    blockhash_resp = client.get_latest_blockhash()
    blockhash = blockhash_resp.value.blockhash

    msg = Message.new_with_blockhash(
        all_ixs,
        payer.pubkey(),
        blockhash,
    )
    tx = Transaction.new_unsigned(msg)
    tx.sign([payer], blockhash)

    result = client.send_transaction(tx)
    sig = str(result.value)

    client.confirm_transaction(result.value, commitment=Confirmed)

    return sig, str(pda)
