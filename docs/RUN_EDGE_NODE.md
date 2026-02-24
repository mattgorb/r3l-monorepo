# Running an Edge Node

The edge node allows you to verify files locally and submit attestations to R3L via API key. The cloud API handles Solana transactions on your behalf.

Three client implementations are available in `services/edge-nodes/`:

| Client | Directory | Best for |
|--------|-----------|----------|
| **Python** | `services/edge-nodes/python/` | Developers, scripting, pip-installable SDK |
| **Rust** | `services/edge-nodes/rust/` | Production nodes, single static binary, cross-compile |
| **C** | `services/edge-nodes/c/` | IoT/embedded devices, minimal footprint |

## Prerequisites

- The R3L API running (locally via `./dev.sh` or at your deployment URL)
- The R3L verifier binary built (Python and Rust clients call it for C2PA verification)

## Environment Variables (all clients)

| Variable | Default | Description |
|----------|---------|-------------|
| `R3L_API_URL` | `http://localhost:3001` | API base URL |
| `R3L_API_KEY` | — | API key from registration |
| `R3L_KEYPAIR` | `edge-keypair.json` | Path to Ed25519 keypair |

---

## Python Client

### Install

```bash
cd services/edge-nodes/python
pip install .
```

This installs the `r3l-edge` command and the `r3l_edge` Python package.

### CLI Usage

```bash
# 1. Register — generates keypair + gets API key
r3l-edge register --name my-node

# 2. Save your API key
export R3L_API_KEY=<key from step 1>

# 3. Attest a file (verifies locally, submits to chain)
r3l-edge attest path/to/image.jpg

# 4. Query the trust verdict
r3l-edge query <content_hash>
```

### SDK Usage (programmatic)

```python
from r3l_edge.client import R3LEdgeClient

client = R3LEdgeClient(api_url="https://your-api.example.com")

# First time: generate keypair and register
client.generate_keypair("edge-keypair.json")
resp = client.register(name="my-sensor")
print(f"API Key: {resp['api_key']}")

# Subsequent runs: load existing keypair
client = R3LEdgeClient(
    api_url="https://your-api.example.com",
    api_key="your-api-key",
    keypair_path="edge-keypair.json",
)

# Verify a file locally and submit attestation
result = client.verify_and_attest("photo.jpg", verifier_bin="verifier", trust_dir="data/trust")
print(result)

# Or attest a pre-computed hash directly
result = client.attest(content_hash="abc123...", has_c2pa=False)

# Query trust verdict
verdict = client.query("abc123...")
print(verdict["verdict"])  # "trusted", "attested", or "unknown"
```

### CLI Commands

| Command | Description |
|---------|-------------|
| `r3l-edge register` | Generate keypair, register with API, get API key |
| `r3l-edge attest <file>` | Verify file locally + submit attestation on-chain |
| `r3l-edge lookup <hash>` | Raw attestation data by content hash |
| `r3l-edge query <hash>` | Structured trust verdict (v1 API) |

---

## Rust Client

### Build

```bash
cd services/edge-nodes/rust
cargo build --release
```

Binary is at `target/release/r3l-edge` (~5MB, statically linked, no runtime deps).

### Cross-compile (e.g. for ARM Linux)

```bash
# Install cross-compilation toolchain
cargo install cross

# Build for ARM (Raspberry Pi, etc.)
cross build --release --target aarch64-unknown-linux-gnu

# Build for MIPS (routers, etc.)
cross build --release --target mips-unknown-linux-gnu
```

### Usage

```bash
# Register
./r3l-edge register --name my-node

# Save API key
export R3L_API_KEY=<key from registration>

# Attest a file (runs verifier binary, signs wallet message, submits)
./r3l-edge attest photo.jpg --verifier /path/to/verifier --trust-dir /path/to/trust

# Hash a file without submitting
./r3l-edge hash photo.jpg

# Query trust verdict
./r3l-edge query <content_hash>

# Raw lookup
./r3l-edge lookup <content_hash>
```

### Commands

| Command | Description |
|---------|-------------|
| `r3l-edge register` | Generate keypair, register with API, get API key |
| `r3l-edge attest <file>` | Verify + sign + submit attestation |
| `r3l-edge hash <file>` | SHA-256 hash a file (no network) |
| `r3l-edge query <hash>` | Structured trust verdict |
| `r3l-edge lookup <hash>` | Raw attestation data |

---

## C Client

Minimal client for IoT and embedded devices. Requires `libcurl` and `openssl` (or `mbedtls`).

### Build

```bash
cd services/edge-nodes/c
make
```

Binary is at `r3l-edge` (~50KB).

On Linux with system OpenSSL:
```bash
OPENSSL_PREFIX=/usr CURL_PREFIX=/usr make
```

### Usage

```bash
# Hash a file (SHA-256)
./r3l-edge hash photo.jpg

# Attest (hash + sign + submit)
./r3l-edge attest photo.jpg --api-key <YOUR_KEY>

# With keypair for wallet signing
./r3l-edge attest photo.jpg --api-key <YOUR_KEY> --keypair edge-keypair.json

# Query trust verdict
./r3l-edge query <content_hash>
```

### Commands

| Command | Description |
|---------|-------------|
| `r3l-edge hash <file>` | SHA-256 hash a file |
| `r3l-edge attest <file>` | Hash + sign + submit (no C2PA verification — IoT use case) |
| `r3l-edge query <hash>` | Query trust verdict |

### IoT Notes

The C client does **not** run the C2PA verifier — it hashes the file and submits with `has_c2pa: false`. The device's identity comes from its Ed25519 wallet key, not from C2PA metadata. This is the intended flow for sensors and capture devices that are the *source* of content.

For bare-metal / RTOS (ESP32, etc.), swap OpenSSL for mbedtls:
```bash
gcc -O2 -DR3L_USE_MBEDTLS -o r3l-edge r3l_edge.c main.c -lmbedcrypto -lcurl
```

---

## Keypair Format

All three clients use the same Solana-compatible keypair format: a JSON array of 64 bytes `[secret_32_bytes..., public_32_bytes...]`.

```bash
# Generate with Python
python3 -c "
from nacl.signing import SigningKey
import json
key = SigningKey.generate()
open('edge-keypair.json','w').write(json.dumps(list(key.encode() + key.verify_key.encode())))
print('Saved edge-keypair.json')
"

# Or use any client's register command (auto-generates if missing)
r3l-edge register --name my-node
```

A single keypair works across all three clients.

---

## Registration Flow

All clients follow the same registration protocol:

1. Generate Ed25519 keypair (or load existing)
2. Sign the message `"R3L: register"` with the private key
3. POST to `/api/edge/register` with `{pubkey, message, signature, name}`
4. Receive API key (tied permanently to that wallet)

## Attestation Flow

1. Hash the file (SHA-256) — all clients
2. Run verifier binary for C2PA metadata — Python and Rust only (C client skips this)
3. Sign `"R3L: attest <content_hash>"` with wallet key
4. POST to `/api/edge/attest` with verification results + wallet signature + API key
5. API writes attestation to Solana + database

The wallet signature is verified on-chain via the Ed25519 precompile, making it independently verifiable by anyone.

---

## Query API Response

The `/api/v1/query/{hash}` endpoint returns a structured trust verdict:

```json
{
  "version": "1.0",
  "content_hash": "abc123...",
  "verdict": "trusted",
  "c2pa": {
    "present": true,
    "trust_list": "official",
    "validation_state": "Trusted",
    "digital_source_type": "https://cv.iptc.org/.../trainedAlgorithmicMedia",
    "source_type_label": "AI-Generated"
  },
  "identity": {
    "email_domain": "example.com",
    "wallet_pubkey": "JAqstf...",
    "wallet_verified_onchain": true
  },
  "signer": {
    "issuer": "Adobe Inc",
    "common_name": "Adobe Firefly",
    "software_agent": "Adobe Firefly 1.0",
    "signing_time": "2025-01-15T12:00:00Z"
  },
  "proof": {
    "type": "trusted_verifier",
    "on_chain": true,
    "pda": "...",
    "tx_signature": "..."
  },
  "attestation": {
    "timestamp": 1705320000,
    "submitted_by": "my-edge-node",
    "verifier_version": "0.1.0",
    "trust_bundle_hash": "abc..."
  }
}
```

| Verdict | Meaning |
|---------|---------|
| `trusted` | C2PA metadata present + signer on official/curated trust list |
| `attested` | On-chain attestation exists (may have wallet/email identity, may lack trusted C2PA) |
| `unknown` | No attestation found for this content hash |
