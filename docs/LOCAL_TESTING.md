# Local Testing Guide

## Quick Start

One command to run everything:

```bash
./dev.sh
```

Open **http://localhost:3001** — done.

The script:
1. Builds the Solana program if needed (`anchor build`)
2. Starts `solana-test-validator` with the program pre-loaded
3. Starts the API + frontend via `docker compose up api`
4. Ctrl+C stops everything

### Prerequisites

| Tool | Version | Install |
|------|---------|---------|
| Docker | 24+ | [docker.com](https://www.docker.com/) |
| Rust | stable 1.82+ | [rustup.rs](https://rustup.rs/) |
| Solana CLI | 2.x or 3.x | `sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)"` |
| Anchor CLI | 0.30.x | `cargo install --git https://github.com/coral-xyz/anchor avm && avm install 0.30.1 && avm use 0.30.1` |

Add Solana CLI to your PATH (add to `~/.zshrc` or `~/.bashrc`):
```bash
export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"
```

---

## Test the Flow

### Via the UI

1. Go to http://localhost:3001/verify
2. Drop a media file (try `data/samples/chatgpt.png`)
3. See the verification report card with trust tier
4. Click **"Attest on Solana"** — submits to the local validator
5. Click the attestation link or go to `/lookup` and paste the content hash

### Via curl

```bash
# Health check
curl http://localhost:3001/api/health

# Verify a file (no Solana submission)
curl -X POST http://localhost:3001/api/verify \
  -F "file=@data/samples/chatgpt.png"

# Verify + attest on Solana
curl -X POST http://localhost:3001/api/attest \
  -F "file=@data/samples/chatgpt.png"

# Look up an attestation by content hash
curl http://localhost:3001/api/attestation/<content_hash_hex>
```

---

## Development Mode (with hot reload)

If you're working on the frontend and want instant hot reload:

```bash
# Terminal 1 — validator + API
./dev.sh

# Terminal 2 — frontend with hot reload
cd services/web
npm install
npm run dev
```

Open **http://localhost:5173** (Vite dev server). It proxies `/api/*` to the API on port 3001.

---

## Solana Program Tests

Anchor/Mocha integration tests that run against the local validator.

```bash
# Make sure the validator is running (./dev.sh or solana-test-validator)
cd services/provenance_attestation
npx mocha --timeout 60000 tests/provenance_attestation.ts
```

**What the tests cover:**
- `submit_proof` — stores an attestation from a (mock) ZK proof
- Duplicate rejection — same content hash can't be attested twice
- Content hash mismatch — proof's content hash must match the instruction argument
- String length validation — rejects strings exceeding 128 bytes

> The tests use `skip-verification` so they pass empty proof buffers. They don't currently test `submit_attestation` (trusted verifier path).

---

## Standalone Tools

### Verifier (no Solana needed)

```bash
cd services/verifier

# With trust lists
TRUST_DIR=../../data/trust cargo run -- ../../data/samples/chatgpt.png

# Without trust lists (everything shows as "untrusted")
cargo run -- ../../data/samples/chatgpt.png

# Via Docker
docker compose run --rm --build verifier /data/samples/chatgpt.png
```

### Prover (requires SP1 toolchain)

```bash
cd services/prover/script

# Mock mode (instant, no real proof)
cargo run --release --bin prove -- \
  --media ../../../data/samples/chatgpt.png \
  --trust-dir ../../../data/trust \
  --mock

# Real Groth16 proof (CPU: 10-30 min, GPU: 5-15 min)
cargo run --release --bin prove -- \
  --media ../../../data/samples/chatgpt.png \
  --trust-dir ../../../data/trust

# Print the verification key hash
cargo run --bin vkey
```

---

## Environment Variables

The API reads these at startup. `dev.sh` and Docker Compose set them automatically — you only need these for manual runs.

| Variable | Default | Description |
|----------|---------|-------------|
| `TRUST_DIR` | `../../data/trust` | Trust anchor PEM certificate directory |
| `PROVER_DIR` | `../prover` | Path to prover cargo project |
| `SOLANA_RPC_URL` | `http://127.0.0.1:8899` | Solana RPC endpoint |
| `SOLANA_KEYPAIR_PATH` | `~/.config/solana/id.json` | Keypair for signing transactions |
| `PROGRAM_ID` | *(placeholder)* | Deployed Solana program address |
| `STATIC_DIR` | `./static` | Frontend build output (production) |
| `BIND_ADDR` | `0.0.0.0:3001` | API listen address |

---

## Troubleshooting

**`anchor build` fails with IDL errors**
Use `anchor build --no-idl`. The IDL is checked in at `services/provenance_attestation/target/idl/provenance_attestation.json`.

**`solana-test-validator: command not found`**
Add the Solana CLI to your PATH:
```bash
export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"
```

**`anchor build` hangs with "Blocking waiting for file lock on build directory"**
Another cargo process (likely rust-analyzer in VS Code) holds the lock. Kill it:
```bash
pkill -f "cargo|rust-analyzer"
```

**`Account does not exist` when looking up an attestation**
The file hasn't been attested yet. Use `/api/attest` or the Verify page first.

**`already in use` error when attesting**
An attestation already exists for that content hash. Each file can only be attested once. Restart `./dev.sh` (the `--reset` flag wipes the ledger).

**API can't connect to Solana**
Make sure the validator is running. `dev.sh` starts it automatically. If running manually, check that `solana-test-validator` is up on port 8899.

**`blake3` build error (edition2024)**
The workspace `Cargo.toml` pins `blake3 = ">=1.3.1, <1.8"`. Make sure you're building from `services/provenance_attestation/`.
