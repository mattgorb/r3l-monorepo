# End-to-End Testing Guide

This guide walks through running the full r3l-provenance pipeline locally: **Upload -> Verify -> Prove -> Submit to Solana -> Lookup**.

## Architecture Overview

```
Browser (localhost:5173)
   │
   ▼
Vue Frontend ──proxy──> API Server (localhost:3001)
                           │
                ┌──────────┼──────────┐
                ▼          ▼          ▼
           Verifier    SP1 Prover  Solana RPC
           (library)   (cargo)    (localhost:8899)
                                      │
                                      ▼
                              Solana Program
                          (provenance_attestation)
```

## Prerequisites

- Rust toolchain (stable)
- Node.js 18+
- Solana CLI (`solana`, `solana-test-validator`)
- Anchor CLI 0.30.x
- SP1 SDK 5.x (for proof generation)

### Solana CLI Path

If `solana` isn't in your PATH, export it:

```bash
export PATH="$HOME/.local/share/solana/install/releases/stable-*/solana-release/bin:$PATH"
```

---

## Step 1: Start the Solana Validator

Open a dedicated terminal:

```bash
# Runs a local Solana blockchain on your machine (like Hardhat/Ganache for Ethereum)
# --reset wipes previous state so you start fresh
solana-test-validator --reset
```

This runs on `localhost:8899`. Leave it running.

## Step 2: Deploy the Solana Program

In another terminal:

```bash

export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"

cd services/provenance_attestation

# Point Solana CLI at the local validator
solana config set --url http://127.0.0.1:8899

# Get free SOL on localnet (needed to pay for transactions + deploying)
solana airdrop 10

# Compile the Rust smart contract to a .so binary that runs on the Solana VM
# --no-idl: skip IDL generation (broken on Anchor 0.30)
# --features skip-verification: disables on-chain proof verification (for mock proofs)
anchor build --no-idl -- --features skip-verification

# Upload the compiled program to the local blockchain
# --program-id uses a keypair file to give it a deterministic address
# (without this, you'd get a random address each deploy)
solana program deploy target/deploy/provenance_attestation.so \
  --program-id target/deploy/provenance_attestation-keypair.json
```

Note the Program ID printed (e.g., `kK63ncGUJXCWjUqSYzcN92tW531rt4UtacJBmHWXJ16`). You'll need it for the API server.

## Step 3: Start the API Server

```bash
cd services/api

TRUST_DIR=../../data/trust \
PROGRAM_ID=kK63ncGUJXCWjUqSYzcN92tW531rt4UtacJBmHWXJ16 \
cargo run
```

The API starts on `localhost:3001`.

### API Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `TRUST_DIR` | *(required)* | Path to trust anchor PEM files |
| `PROGRAM_ID` | `HahVgC9...` (old) | Solana program ID — **set this to your deployed ID** |
| `SOLANA_RPC_URL` | `http://127.0.0.1:8899` | Solana RPC endpoint |
| `SOLANA_KEYPAIR_PATH` | `~/.config/solana/id.json` | Wallet keypair for transactions |
| `PROVER_DIR` | `../prover` | Path to SP1 prover project |
| `PROVER_MOCK` | `true` | Set to `false` for real Groth16 proofs |
| `BIND_ADDR` | `0.0.0.0:3001` | Listen address |

## Step 4: Start the Web Frontend

```bash
cd services/web
npm install   # first time only
npm run dev
```

Opens on `http://localhost:5173`. The Vite dev server proxies `/api/*` to `localhost:3001`.

---

## Using the Web UI

### Verify a File

1. Open http://localhost:5173
2. Drag and drop (or click to select) a media file with C2PA metadata
   - Sample file: `data/samples/chatgpt.png`
3. The UI calls `POST /api/verify` and displays:
   - Trust badge (official / curated / untrusted)
   - C2PA fields: issuer, signing time, software agent, digital source type, etc.

### Generate a ZK Proof

1. After verification, click **"Generate Proof"**
2. The API shells out to the SP1 prover (mock mode by default for speed)
3. Returns proof bytes + public outputs (hex-encoded)
4. The UI shows a proof preview

### Submit to Solana

1. After proof generation, click **"Submit"**
2. The API constructs a Solana transaction with the Groth16 proof
3. Submits to localnet, creates an on-chain attestation PDA
4. The UI shows the transaction signature and PDA address

### Lookup an Attestation

1. Enter a content hash (hex) in the lookup form
2. The UI calls `GET /api/attestation/{hash}`
3. Displays stored on-chain data: issuer, trust match, signing time, etc.

---

## CLI Testing (No UI)

### Verify only

```bash
cd services/verifier
TRUST_DIR=../../data/trust cargo run -- ../../data/samples/chatgpt.png
```

### Prove (mock)

```bash
cd services/prover
cargo run --bin prove -- --media ../../data/samples/chatgpt.png --trust-dir ../../data/trust --mock
```

### API health check

```bash
curl http://localhost:3001/api/health
# → "ok"
```

### API verify

```bash
curl -F "file=@data/samples/chatgpt.png" http://localhost:3001/api/verify | jq .
```

### API prove

```bash
curl -F "file=@data/samples/chatgpt.png" http://localhost:3001/api/prove | jq .
```

### Solana program tests

```bash
cd services/provenance_attestation

# Start validator + deploy first (steps 1-2 above), then:
ANCHOR_PROVIDER_URL=http://localhost:8899 \
ANCHOR_WALLET=~/.config/solana/id.json \
npx mocha --timeout 60000 'tests/**/*.ts'
```

Expected output:
```
  provenance_attestation
    ✔ submits a proof and stores an attestation
    ✔ rejects duplicate attestation for same content hash
    ✔ rejects content_hash mismatch
    ✔ rejects strings that exceed max length

  4 passing
```

---

## Troubleshooting

| Problem | Fix |
|---------|-----|
| `solana: command not found` | Export the Solana CLI path (see Prerequisites) |
| `ANCHOR_PROVIDER_URL is not defined` | Set env vars before running mocha tests |
| `anchor build` IDL error | Use `anchor build --no-idl` (known Anchor 0.30 issue) |
| API returns wrong program ID error | Set `PROGRAM_ID` env var to match your deployed program |
| `Account does not exist` on lookup | The attestation hasn't been submitted yet, or validator was reset |
| Proof generation is slow | Mock mode is fast; real Groth16 proofs take 10-30 min on CPU |
| `already in use` on submit | Attestation for this content hash already exists; reset validator and redeploy |

---

## Where Data Lives

The proof and attestation data flows through several layers:

| Layer | What's stored | Persistence |
|-------|--------------|-------------|
| **Browser** (Vue state) | Verify results, proof hex, public_values hex | In-memory only — lost on page refresh |
| **API server** | Nothing — stateless, temp files cleaned up | None |
| **Solana blockchain** | Attestation PDA (content hash, issuer, trust match, etc.) | Permanent (until validator reset on localnet) |

The browser holds proof data in Vue component refs (`proofHex`, `publicInputsHex`) and passes them between components via props/events. Nothing is persisted to disk or localStorage on the frontend. If you refresh the page, you need to re-verify and re-prove.

The on-chain attestation is the permanent record. Once submitted, it can be looked up by anyone using the content hash.

---

## AWS GPU Proving (Real Groth16 Proofs)

This section covers running the prover on a GPU EC2 instance to generate real (non-mock) Groth16 proofs. Real proofs are cryptographically valid and can be verified on-chain without the `skip-verification` feature flag.

### What changes from local

| Component | Local (mock) | AWS (real) |
|-----------|-------------|------------|
| **Prover** | `--mock` flag, instant | CUDA GPU, ~5-15 min per proof |
| **Solana program** | `skip-verification` feature | Full `verify_proof()` on-chain |
| **Solana network** | `solana-test-validator` (local) | Devnet or Mainnet |

### Instance selection

| Instance | Cost | Proof time | Notes |
|----------|------|------------|-------|
| `g4dn.xlarge` | ~$0.50/hr | ~5-15 min | Recommended — NVIDIA T4 GPU |
| `c5.4xlarge` | ~$0.70/hr | ~10-30 min | CPU only, no GPU setup |

Use the **Deep Learning AMI** (Amazon Linux 2) — comes with NVIDIA drivers + CUDA pre-installed.

### One-time EC2 setup

SSH into the instance and install toolchains:

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Install SP1 toolchain (only need to run once, persists across reboots)
curl -L https://sp1.succinct.xyz | bash
source $HOME/.sp1/bin/env
sp1up

# Install Solana CLI
sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)"
export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"

# anchor
cargo install --git https://github.com/coral-xyz/anchor --tag v0.30.1 anchor-cli

#frontend
curl -fsSL https://rpm.nodesource.com/setup_22.x | sudo bash -
sudo yum install -y nodejs

# Verify CUDA (should show NVIDIA T4 or similar)
nvidia-smi
```

Add to `~/.bashrc` so paths persist across sessions:

```bash
echo 'source $HOME/.cargo/env' >> ~/.bashrc
echo 'source $HOME/.sp1/bin/env' >> ~/.bashrc
echo 'export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"' >> ~/.bashrc
```

### Sync code to EC2

From your local machine:

```bash
# Sync source code (excludes build artifacts)
rsync -avz \
  -e "ssh -i ~/.ssh/id_rsa" \
  --exclude target \
  --exclude node_modules \
  --exclude test-ledger \
  --exclude .anchor \
  ~/Desktop/projects/r3l-provenance/services/ \
  ec2-user@18.209.36.61:~/r3l-provenance/services/


# Sync data (trust anchors + sample media)
rsync -avz \
  ~/Desktop/projects/r3l-provenance/data/ \
  ec2-user@100.31.50.216:~/r3l-provenance/data/
  
```

### Build on EC2

After syncing code, rebuild everything that changed:

```bash
# 1. Prover binary (SP1 guest + host) — only if prover/ source changed
cd ~/r3l-provenance/services/prover/script
cargo build --release

# 2. Solana program — only if provenance_attestation/ source changed
cd ~/r3l-provenance/services/provenance_attestation
anchor build --no-idl
# Redeploy to running validator (airdrop first if validator was reset)
solana airdrop 10
solana program deploy target/deploy/provenance_attestation.so \
  --program-id target/deploy/provenance_attestation-keypair.json

# 3. Vue frontend — only if web/ source changed
cd ~/r3l-provenance/services/web
npm install
npm run build

# 4. API server — only if api/ or verifier/ source changed
cd ~/r3l-provenance/services/api
cargo build --release
```

First prover build takes ~5 min (compiles the SP1 guest program + host binary). Subsequent builds are incremental.

### Generate a real Groth16 proof

```bash
cd ~/r3l-provenance/services/prover/script

SP1_PROVER=cuda ../target/release/prove \
  --media ~/r3l-provenance/data/samples/chatgpt.png \
  --trust-dir ~/r3l-provenance/data/trust \
  --json-out proof-output.json
```

The `--json-out` flag writes a JSON sidecar with hex-encoded `proof` and `public_values` — this is what gets submitted to Solana.

Expected output:

```
Detected format: PNG
Found C2PA JUMBF data: 96217 bytes
Extracted claim (648 bytes), signature (12991 bytes), 6 assertion(s)
Extracted 2 certificate(s) from COSE x5chain
...
--- Public Outputs ---
content_hash: 4ebd98d3893a16a6b3cf73c4b3cdf3b55149af563c47e07dd58f9bba17a8aabf
has_c2pa: true
trust_list_match: curated
validation_state: Verified
issuer: Truepic
common_name: Truepic Lens CLI in Sora
software_agent: ChatGPT
digital_source_type: http://cv.iptc.org/newscodes/digitalsourcetype/trainedAlgorithmicMedia
cert_fingerprint: f5630fc1618418635536cd5c88377b96958d2d428e8b7f4ad08fcab1b58559ab
generating Groth16 proof...
proof verified locally
proof saved to proof.bin
proof bytes: 260 bytes
public values: 284 bytes
JSON sidecar written to proof-output.json
```

### Supported media formats

| Format | Detection | C2PA extraction | End-to-end (has_c2pa: true) |
|--------|-----------|----------------|---------------------------|
| **PNG** | `\x89PNG` magic | caBX chunks | Yes (ES256 signatures) |
| **JPEG** | `\xFF\xD8\xFF` magic | APP11 marker segments | Only ES256 — Adobe files use PS256, so `has_c2pa: false` |
| **MP4** | `ftyp` at offset 4 | UUID box with C2PA UUID | Yes (ES256 signatures) |
| **PDF** | `%PDF-` magic | Associated File with `/AFRelationship /C2PA_Manifest` | Yes (ES256 signatures) |

The guest program only verifies **ES256 (P-256 ECDSA)** signatures. Files signed with PS256 (RSA-PSS) will extract correctly but output `has_c2pa: false` because the signature can't be verified in the zkVM.

### Test data that works end-to-end

```bash
# PNG — ChatGPT/OpenAI (ES256, curated trust)
SP1_PROVER=cuda ../target/release/prove \
  --media ~/r3l-provenance/data/samples/chatgpt.png \
  --trust-dir ~/r3l-provenance/data/trust

# MP4 — C2PA test video (ES256, untrusted test CA)
SP1_PROVER=cuda ../target/release/prove \
  --media ~/r3l-provenance/data/samples/park-bench-sunset-c2pa-max-vmh.mp4 \
  --trust-dir ~/r3l-provenance/data/trust
```

Files that extract but produce `has_c2pa: false` (PS256 signatures):

```bash
# JPEG — Adobe test file (PS256, signature verification skipped)
SP1_PROVER=cuda ../target/release/prove \
  --media ~/r3l-provenance/data/fixtures/c2pa-public-testfiles/legacy/1.4/image/jpeg/adobe-20220124-C.jpg \
  --trust-dir ~/r3l-provenance/data/trust
```

### Deploy the Solana program on EC2

The Solana program runs on a Solana cluster, not in Docker. For testing on EC2, run a local validator:

```bash
# Terminal 1 — start a local Solana blockchain (leave running)
solana-test-validator --reset

# Terminal 2 — deploy
cd ~/r3l-provenance/services/provenance_attestation

# Point CLI at the local validator
solana config set --url http://127.0.0.1:8899
# Get free SOL for transaction fees
solana airdrop 10

# Compile the smart contract — NO skip-verification so real GPU proofs
# are cryptographically verified on-chain (only needed once, unless source changes)
anchor build --no-idl

# Upload the compiled .so to the blockchain with a deterministic address
solana program deploy target/deploy/provenance_attestation.so \
  --program-id target/deploy/provenance_attestation-keypair.json
```

#### After a validator restart
You don't need to rebuild — just re-fund and re-deploy the existing `.so`:
```bash
solana-test-validator --reset

#separate terminaL
cd ~/r3l-provenance/services/provenance_attestation
anchor build --no-idl
solana airdrop 10
solana program deploy target/deploy/provenance_attestation.so \
  --program-id target/deploy/provenance_attestation-keypair.json
```

Note the Program ID printed — you'll need it for the API.

### Run API + UI + verifier on EC2

The API server bundles the verifier (as a library) and serves the Vue frontend as static files. Run everything natively with cargo — no Docker needed.

```bash
# Install Node.js (for building the frontend)
curl -fsSL https://rpm.nodesource.com/setup_22.x | sudo bash -
sudo yum install -y nodejs


# Build and run the API server (serves frontend from ../web/dist)
# PROVER_MOCK=false: generate real Groth16 proofs (not mock)
# SP1_PROVER=cuda: use GPU for proof generation
cd ~/r3l-provenance/services/api
PROVER_MOCK=false \
SP1_PROVER=cuda \
TRUST_DIR=../../data/trust \
PROGRAM_ID=6VoF5vTQfCSVUMqbpxw8Z8YjvzPEmgkYD6477kyN6eNw \
SOLANA_RPC_URL=http://127.0.0.1:8899 \
STATIC_DIR=../web/dist \
PROVER_MOCK=false \
SP1_PROVER=cuda \
cargo run --release

cd ~/r3l-provenance/services/web
npm install
npm run build
ssh -i ~/.ssh/id_rsa -L 3001:localhost:3001 ec2-user@18.209.36.61
```

The API starts on port 3001, serving both the REST API and the Vue UI.

### Access the UI from your local machine

Use an SSH tunnel to access port 3001 locally:

```bash
# From your local machine
ssh -i ~/.ssh/id_rsa -L 3001:localhost:3001 ec2-user@54.210.48.227
```

Then open **http://localhost:3001** in your browser.

### What runs where

| Component | Where | How |
|-----------|-------|-----|
| **Prover** | EC2 host | `SP1_PROVER=cuda ../target/release/prove ...` |
| **Solana validator** | EC2 host | `solana-test-validator` |
| **Solana program** | On the validator | `solana program deploy` |
| **API + verifier + UI** | EC2 host | `cargo run --release` (in `services/api`) |
| **Your browser** | Local machine | SSH tunnel → `localhost:3001` |
