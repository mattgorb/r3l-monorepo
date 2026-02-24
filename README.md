# R3L Monorepo

R3L is a media attestation platform on Solana. Upload any media file — image, video, PDF — and R3L records a permanent on-chain attestation keyed by its SHA-256 content hash, linked to the attester's verified identity. If the file contains [C2PA](https://c2pa.org/) provenance metadata, R3L also verifies the manifest chain and certificate trust, giving the attestation a higher trust tier. All attested content is indexed with CLIP embeddings and TLSH hashes in pgvector for semantic similarity search.

## How It Works

1. **Attest** — Upload a media file. R3L hashes it, checks for C2PA metadata (if present, validates the manifest chain against official and curated trust lists), and writes the attestation on-chain as a Solana PDA. The attestation is optionally linked to the attester's identity (email, wallet, or org).
2. **Verify** — If the file has C2PA metadata, R3L parses and validates the full manifest chain, certificate trust, digital source type, issuer, and signing time.
3. **Prove (ZK)** — C2PA verification can run inside SP1's zkVM to produce a Groth16 proof verified on-chain. Fully trustless, requires GPU.
4. **Search** — CLIP embeddings and TLSH hashes are stored in pgvector. Query by file upload or content hash to find semantically similar attested media.
5. **Lookup** — Anyone can look up an attestation by content hash to see its trust tier, C2PA signals (if any), attester identity, and on-chain transaction.

### Trust Tiers

| Tier | Label | Criteria |
|------|-------|----------|
| 1 | Verified Origin | C2PA + official trust list + device-signed (`digitalCapture`) |
| 2 | Verified Tool Chain | C2PA + official or curated trust list |
| 3 | Verified Creator | Identity-verified attester in R3L registry, no C2PA required |
| 4 | Unverified | Anonymous attestation, no C2PA, unknown signer, or untrusted certificate |

## Project Structure

```
services/
  api-py/             FastAPI server (verify, attest, prove, search, auth, org, DID)
  api-rust/           Original Axum API (deprecated, kept for reference)
  web/                Vue 3 + TypeScript + Tailwind frontend
  verifier/           Standalone C2PA verifier (Rust lib + CLI)
  prover/             SP1 zkVM prover (Groth16 proof generation)
  provenance_attestation/   Anchor Solana program
  edge-nodes/         Edge node implementations (C, Python, Rust)
data/
  samples/            Test media files
  test_files/         Additional test fixtures
  trust/              Trust anchor PEM certificates (official + curated)
docs/                 Architecture, deployment, local testing, edge node docs
aws-infra/            Terraform configs (App Runner, GPU instances)
docker/               Dockerfiles for API, verifier, validator
```

## Quick Start

**Prerequisites:** Python 3.11+, Rust (stable), Solana CLI 2.x+, Anchor CLI 0.30.x, Node.js 18+, Docker (for Postgres)

The `dev.sh` script starts everything — Postgres, Solana test validator, and the API server:

```bash
./dev.sh
```

This will:
- Build the Vue frontend
- Start a pgvector Postgres container
- Build and deploy the Solana program to a local test validator
- Start the FastAPI server on port 3001

Open **http://localhost:3001** to use the app.

### Manual Setup

**Postgres:**
```bash
docker run -d --name r3l-postgres -p 5432:5432 \
  -e POSTGRES_PASSWORD=postgres -e POSTGRES_DB=r3l \
  pgvector/pgvector:pg16
```

**Solana program:**
```bash
cd services/provenance_attestation
anchor build --no-idl -- --features skip-verification,skip-authority-check
```

**API server:**
```bash
cd services/api-py
pip install -r requirements.txt
PROGRAM_ID=<program-id> \
  TRUST_DIR=../../data/trust \
  DATABASE_URL=postgresql://postgres:postgres@localhost:5432/r3l \
  SOLANA_RPC_URL=http://127.0.0.1:8899 \
  python -m uvicorn main:app --host 0.0.0.0 --port 3001 --reload
```

**Frontend (dev mode):**
```bash
cd services/web
npm install && npm run dev
```

### Docker Compose

```bash
docker compose up
```

Starts Postgres, the API server, and the verifier. Requires a local Solana validator running on port 8899.

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/attest` | Upload file, attest on-chain (with C2PA verification if present) |
| POST | `/api/verify` | Upload file, get C2PA verification report (no on-chain write) |
| GET | `/api/attestation/{hash}` | Look up attestation by content hash |
| GET | `/api/attestations` | List all attestations |
| POST | `/api/prove` | Generate ZK proof of C2PA verification |
| POST | `/api/submit` | Submit pre-generated proof on-chain |
| GET | `/api/v1/query/{hash}` | Query verdict for a content hash |
| POST | `/api/v1/similar` | Find similar media by file upload |
| GET | `/api/v1/similar/{hash}` | Find similar media by hash |
| POST | `/api/auth/email/start` | Start email authentication |
| POST | `/api/auth/email/verify` | Complete email authentication |
| GET | `/api/auth/wallet/challenge` | Get wallet sign challenge |
| POST | `/api/auth/wallet/verify` | Complete wallet authentication |
| GET | `/api/auth/me` | Get current user info |
| POST | `/api/org/register` | Register an organization (DNS or email verification) |
| GET | `/api/did/{did}` | Resolve a DID document |
| GET | `/.well-known/did.json` | Platform DID document |

## Identity & Auth

Accounts support multiple linked identity types:

- **Email** — 6-digit code verification
- **Solana Wallet** — Ed25519 challenge-response signing
- **Organization** — DNS TXT record or admin email verification, with DID issuance (`did:web`, `did:pkh`, `did:jwk`, `did:key`)

Authenticated users receive an API key (`r3l_...`) used via the `X-API-Key` header. Organizations can issue sub-keys to members.

## Documentation

- [Architecture](docs/architecture.md)
- [Local Testing](docs/LOCAL_TESTING.md)
- [Deployment](docs/DEPLOY.md)
- [Running Edge Nodes](docs/RUN_EDGE_NODE.md)
