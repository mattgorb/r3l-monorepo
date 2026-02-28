# R3L — The Trust Layer for Digital Content

R3L is an on-chain attestation platform on Solana. Upload any content — files, URLs, or text — and R3L records a permanent, verifiable attestation keyed by its SHA-256 content hash, linked to the attester's verified identity (wallet, email, or organization). All attested content is indexed with CLIP embeddings and TLSH hashes for similarity search.

## How It Works

1. **Attest** — Submit any content. R3L hashes it and writes an attestation on-chain as a Solana PDA, linked to the attester's verified identity. If the file contains C2PA metadata, it is automatically extracted and included.
2. **Search** — CLIP embeddings and TLSH hashes are stored in pgvector. Query by file upload or content hash to find similar attested content.
3. **Query** — Anyone can look up a content hash and get a structured trust verdict: `trusted`, `attested`, or `unknown` — with full identity and metadata.
4. **Prove (ZK)** — Attestations can optionally be backed by a Groth16 zero-knowledge proof generated inside SP1's zkVM and verified on-chain. Fully trustless, requires GPU.

## For AI Agents

The `/api/v1/query/{hash}` endpoint returns machine-readable trust verdicts. AI agents can:

- **Gate data ingestion** — Hash incoming content, query R3L, and only process data attested by trusted organizations
- **Attest outputs** — Record AI-generated content (reports, summaries, analysis) so downstream consumers can verify origin
- **Chain of custody** — Attest data at each pipeline step, creating an auditable trail from source to output
- **Batch verification** — Query up to 50 hashes in a single call via `POST /api/v1/query/batch`

```bash
# Query a content hash
curl https://api.r3l.com/api/v1/query/a1b2c3...
# → { "verdict": "trusted", "identity": { "org": "reuters.com" }, "proof": { "on_chain": true } }
```

### Trust Tiers

| Tier | Label | Criteria |
|------|-------|----------|
| 1 | Verified Origin | Content attested with C2PA metadata from a trusted device (e.g. camera-signed) |
| 2 | Verified Tool Chain | Content attested with C2PA metadata from a trusted tool or platform |
| 3 | Verified Creator | Identity-verified attester, no embedded C2PA metadata required |
| 4 | Unverified | Anonymous attestation or unrecognized metadata |

## Project Structure

```
services/
  api-py/             FastAPI server (attest, search, query, auth, org, DID, developer API)
  web/                Vue 3 + TypeScript + Tailwind frontend
  verifier/           C2PA metadata verifier (Rust lib + CLI)
  prover/             SP1 zkVM prover (Groth16 proof generation)
  provenance_attestation/   Anchor Solana program
  edge-nodes/         Edge node implementations (Python SDK + CLI)
data/
  samples/            Test media files
  test_files/         Additional test fixtures
  trust/              Trust anchor certificates (official + curated)
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

### Developer API (v1)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/v1/register` | Register account, get API key |
| POST | `/api/v1/verify-identity` | Add email, wallet, or org to account |
| POST | `/api/v1/attest-content` | Attest file, URL, or text |
| POST | `/api/v1/attest-content/batch` | Attest multiple content items at once |
| GET | `/api/v1/query/{hash}` | Query trust verdict for a content hash |
| POST | `/api/v1/query/batch` | Batch query up to 50 hashes |
| POST | `/api/v1/similar` | Find similar content by file upload |
| GET | `/api/v1/similar/{hash}` | Find similar content by hash |
| GET | `/api/v1/me` | Get current account info |
| GET | `/api/content/{hash}` | Retrieve stored content |

### Core API

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/attest` | Upload file, attest on-chain |
| POST | `/api/verify` | Upload file, get C2PA verification report |
| GET | `/api/attestation/{hash}` | Look up attestation by content hash |
| GET | `/api/attestations` | List all attestations |
| POST | `/api/prove` | Generate ZK proof of C2PA verification |
| POST | `/api/submit` | Submit pre-generated proof on-chain |
| POST | `/api/auth/email/start` | Start email authentication |
| POST | `/api/auth/email/verify` | Complete email authentication |
| GET | `/api/auth/wallet/challenge` | Get wallet sign challenge |
| POST | `/api/auth/wallet/verify` | Complete wallet authentication |
| GET | `/api/auth/me` | Get current user info |
| POST | `/api/org/register` | Register an organization |
| GET | `/api/did/{did}` | Resolve a DID document |
| GET | `/.well-known/did.json` | Platform DID document |

### Edge Node CLI

```bash
pip install r3l-edge

r3l-edge register                  # Create account, get API key
r3l-edge attest photo.jpg          # Attest a file (local C2PA + CLIP + TLSH)
r3l-edge attest-url https://...    # Attest a URL
r3l-edge attest-text "..."         # Attest text (or - for stdin)
r3l-edge query <hash>              # Get trust verdict
r3l-edge lookup <hash>             # Get raw attestation record
```

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
