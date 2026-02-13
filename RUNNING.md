# Running the System

## Prerequisites

- Rust (stable, 1.82+)
- Node.js 18+
- Solana CLI 2.x or 3.x (`solana`, `solana-test-validator`)
- Anchor CLI 0.30.x (`anchor`)
- SP1 toolchain (`cargo prove` — install via `sp1up`)

---

## Local Development

#### Build commands: 
```
cd /Users/matt/Desktop/projects/r3l-provenance/services/verifier && cargo build --release
cd /Users/matt/Desktop/projects/r3l-provenance/services/prover/script && cargo build --release
```


### 1. Verifier (standalone)

Verify a file's C2PA metadata. No Solana or ZK needed.

```bash
TRUST_DIR=/Users/matt/Desktop/projects/r3l-provenance/data/trust /Users/matt/Desktop/projects/r3l-provenance/services/verifier/target/release/verifier /Users/matt/Desktop/projects/r3l-provenance/data/samples/chatgpt.png


cd /Users/matt/Desktop/projects/r3l-provenance/services/verifier && cargo build --release
cd services/verifier
TRUST_DIR=../../data/trust cargo run -- ../../data/samples/chatgpt.png
```

Output is JSON with `content_hash`, `has_c2pa`, `trust_list_match`, `issuer`, etc.

To verify a file without trust lists (everything will be `untrusted`):

```bash
cargo run -- /path/to/any/image.png
```

### 2. Prover (standalone)

Generate a ZK proof. Requires verifier output as JSON input.

```bash
cd services/prover/script


# Step 1: Generate verifier JSON output
(cd ../../verifier && TRUST_DIR=../../data/trust cargo run -- ../../data/samples/chatgpt.png 2>/dev/null) > verify.json

# Step 2: Mock mode (instant, no real crypto — for testing)
cargo run --bin prove -- --file verify.json --media ../../../data/samples/chatgpt.png --mock

# Step 3: Real Groth16 proof (CPU: 10-30 minutes, GPU: 5-15 minutes)
cargo run --bin prove -- --file verify.json --media ../../../data/samples/chatgpt.png

# Print the verification key hash:
cargo run --bin vkey
```

**Notes:**
- Mock mode skips the actual Groth16 proving and generates a dummy proof. The guest program still runs and verifies SHA-256.
- The prover **auto-detects GPU** (CUDA) and uses it if available. CPU fallback is automatic.
- For GPU proving on AWS, see `infra/gpu-prover/` Terraform configuration.

### 3. Solana Program (localnet)

Deploy and test the on-chain program.

**Terminal 1 — start local validator:**
```bash
solana-test-validator --reset
```

**Terminal 2 — deploy:**
```bash
cd services/provenance_attestation

# Make sure your Solana CLI points to localnet
solana config set --url http://127.0.0.1:8899

# Airdrop SOL for deployment + transactions
solana airdrop 10

# Build the program (skip IDL generation — it's already checked in)
anchor build --no-idl

# Deploy
solana program deploy target/deploy/provenance_attestation.so \
  --program-id target/deploy/provenance_attestation-keypair.json
```

**Run tests:**
```bash
cd services/provenance_attestation
npx mocha --timeout 60000 tests/provenance_attestation.ts
# → 3 passing
```

**Submit and query via CLI:**
```bash
npx ts-node scripts/submit.ts --proof ../prover/proof.bin
npx ts-node scripts/submit.ts --query <content_hash_hex>
```

### 4. Web API

Serves the HTTP API on port 3001.

```bash
cd services/api

# Required: TRUST_DIR points to the PEM trust anchor directory
# Optional: SOLANA_RPC_URL (default: http://127.0.0.1:8899)
# Optional: SOLANA_KEYPAIR_PATH (default: ~/.config/solana/id.json)
# Optional: PROVER_DIR (default: ../prover)
# Optional: STATIC_DIR (default: ./static — for serving the built Vue app)
# Optional: BIND_ADDR (default: 0.0.0.0:3001)

TRUST_DIR=../../data/trust cargo run
```

Test with curl:

```bash
# Health check
curl http://localhost:3001/api/health

# Verify a file
curl -X POST http://localhost:3001/api/verify \
  -F "file=@../../data/samples/chatgpt.png"

# Look up an attestation (requires Solana localnet running + attestation submitted)
curl http://localhost:3001/api/attestation/<content_hash_hex>
```

### 5. Web UI (dev mode)

Runs the Vue dev server on port 5173 with hot reload. Proxies `/api/*` to the API server.

```bash
cd services/web

npm install    # first time only
npm run dev
# → http://localhost:5173
```

The API server (step 4) must be running for the UI to work.

### 6. Full Local Stack

Run everything together:

**Terminal 1 — Solana validator:**
```bash
solana-test-validator --reset
```

**Terminal 2 — Deploy program (once):**
```bash
cd services/provenance_attestation
solana airdrop 10
solana program deploy target/deploy/provenance_attestation.so \
  --program-id target/deploy/provenance_attestation-keypair.json
```

**Terminal 3 — API server:**
```bash
cd services/api
TRUST_DIR=../../data/trust cargo run
```

**Terminal 4 — Vue dev server:**
```bash
cd services/web
npm run dev
```

Open http://localhost:5173. Upload a file, see C2PA results, submit to Solana, look up by hash.

### Docker (API only)

Build and run the API (includes the built Vue app):

```bash
docker compose up api
```

This builds the API binary and Vue frontend in a multi-stage Docker build. The API serves the Vue app on port 3001. Requires a Solana RPC endpoint accessible from the container (defaults to `host.docker.internal:8899`).

---

## Remote Deployment (Outline)

Not yet implemented. Here's the plan:

### Solana Program

1. **Build for mainnet/devnet:**
   ```bash
   anchor build --no-idl
   ```

2. **Deploy to devnet first:**
   ```bash
   solana config set --url https://api.devnet.solana.com
   solana airdrop 5  # devnet SOL
   solana program deploy target/deploy/provenance_attestation.so \
     --program-id target/deploy/provenance_attestation-keypair.json
   ```

3. **Deploy to mainnet:**
   ```bash
   solana config set --url https://api.mainnet-beta.solana.com
   # Fund the deployer wallet with real SOL
   solana program deploy target/deploy/provenance_attestation.so \
     --program-id target/deploy/provenance_attestation-keypair.json
   ```

4. **Update the program ID** in `constants.rs`, `Anchor.toml`, and the API's `PROGRAM_ID` env var if it changes.

### API + Web UI

1. **Build the production Docker image:**
   ```bash
   docker build -f docker/api.Dockerfile -t provenance-api .
   ```

2. **Push to a container registry** (e.g. Docker Hub, ECR, GCR):
   ```bash
   docker tag provenance-api registry.example.com/provenance-api:latest
   docker push registry.example.com/provenance-api:latest
   ```

3. **Deploy to a cloud provider.** Options:
   - **Fly.io** — `fly launch`, single command deploy, good for small projects
   - **Railway / Render** — connect git repo, auto-deploy on push
   - **AWS ECS / GCP Cloud Run** — container-based, auto-scaling
   - **VPS (DigitalOcean, Hetzner)** — docker compose on a VM, cheapest option

4. **Environment variables to configure:**
   ```
   TRUST_DIR=/data/trust
   SOLANA_RPC_URL=https://api.mainnet-beta.solana.com  (or devnet)
   SOLANA_KEYPAIR_PATH=/keys/id.json
   PROGRAM_ID=HahVgC9uo73aLw1ouBEvgMT7KmGTS6rovfbKP9zuCtjc
   BIND_ADDR=0.0.0.0:3001
   STATIC_DIR=/app/static
   ```

5. **Mount or bake in the trust anchor PEMs** — the `data/trust/` directory with `official/` and `curated/` subdirs containing `.pem` files.

6. **Mount the Solana keypair** — the submitter keypair needs SOL for transaction fees.

7. **Reverse proxy** — put nginx or Caddy in front for TLS:
   ```
   server {
       listen 443 ssl;
       server_name provenance.example.com;
       location / { proxy_pass http://localhost:3001; }
   }
   ```

### Prover (GPU Instance)

The prover is compute-intensive and benefits from GPU acceleration. **See `infra/gpu-prover/` for Terraform configuration.**

**Deployment options:**

1. **AWS GPU Instance (Recommended for production)**
   - Use the Terraform config in `infra/gpu-prover/`
   - Deploys g4dn.xlarge on-demand instance (NVIDIA T4, 16GB VRAM)
   - Cost: $0.526/hour (cheapest GPU instance on AWS)
   - Proof time: 5-15 minutes with GPU vs 10-30 minutes on CPU
   - See `infra/gpu-prover/README.md` for full instructions

2. **Succinct Network (Easiest for small-scale)**
   - Use SP1's hosted proving network
   - Change `.cpu()` to `.network()` in prove.rs
   - Set `SP1_PRIVATE_KEY` environment variable
   - Get API key from https://network.succinct.xyz/
   - Pay per proof, no infrastructure to manage

3. **Local CLI tool**
   - Users generate proofs locally and submit them
   - Requires users to have GPU or tolerate slow CPU proving

**If running as a remote service**, the API's `/api/prove` endpoint should be async:
   - `POST /api/prove` → returns a `job_id`
   - `GET /api/prove/{job_id}` → polls for status + result
   - Backend submits proving jobs to a worker queue (Redis, SQS, etc.)
   - Workers run on GPU instances, process the queue
