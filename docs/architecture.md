# R3L Architecture

## Overview

R3L is a media provenance verification system. Users upload media files, the system extracts and verifies C2PA metadata, optionally links creator identities (email domain, Solana wallet), and records everything as on-chain attestations on Solana. A lookup API allows anyone to query attestations by content hash.

---

## Canonical Output: The Attestation Record

Every file produces up to **three independent on-chain records**, each stored as a Solana PDA (Program Derived Address):

### 1. C2PA Attestation

**PDA seed**: `[b"attestation", sha256(file_bytes)]`

The core record. Contains the raw verification signals extracted from the file's C2PA manifest:

| Field | Example | What it means |
|-------|---------|---------------|
| `content_hash` | `a1b2c3...` (32 bytes) | SHA-256 of the raw file bytes. Primary key for everything. |
| `has_c2pa` | `true` / `false` | Whether the file contained any C2PA provenance metadata. |
| `trust_list_match` | `"official"`, `"curated"`, `"untrusted"` | Whether the signing certificate appears on the C2PA official trust list, the R3L curated trust list, or neither. |
| `validation_state` | `"Trusted"`, `"Valid"`, `"Invalid"` | C2PA SDK validation result for the manifest chain. |
| `digital_source_type` | IPTC URI | How the content was created: digital capture, AI-generated, composite, etc. |
| `issuer` | `"DigiCert"` | Organization that issued the signing certificate. |
| `common_name` | `"Leica M11"` | Certificate common name — typically the device or software. |
| `software_agent` | `"Adobe Photoshop 25.0"` | Tool that wrote the C2PA manifest. |
| `signing_time` | `"2025-01-15T12:00:00Z"` | When the manifest was signed. |
| `cert_fingerprint` | `"d4e5f6..."` | SHA-256 of the leaf signing certificate (for deduplication/revocation checks). |
| `proof_type` | `"trusted_verifier"` or `"zk_groth16"` | How the attestation was created (see Proof Modes below). |
| `submitted_by` | Solana pubkey | The authority that submitted the transaction. |
| `timestamp` | Unix timestamp | Solana clock time when the PDA was created. |

**Who creates it**: The `/api/attest` endpoint (web UI) or `/api/edge/attest` endpoint (edge nodes).

### 2. Identity Attestation (Email)

**PDA seed**: `[b"identity", sha256(file_bytes), domain.as_bytes()]`

Links a verified email domain to the file. One per (file, domain) pair.

| Field | Example | What it means |
|-------|---------|---------------|
| `content_hash` | `a1b2c3...` | Same SHA-256 hash as the C2PA attestation. |
| `domain` | `"washingtonpost.com"` | The verified email domain. |
| `email_hash` | 32 bytes | SHA-256 of the full email address (privacy-preserving). |
| `proof_type` | `"email_domain"` | Always this value. |
| `submitted_by` | Solana pubkey | R3L authority. |
| `timestamp` | Unix timestamp | When recorded. |

**Who creates it**: The `/api/verify-email/attest` endpoint, after the user clicks a verification link sent to their email. The server verifies the user controls an email at that domain, then the R3L authority submits the on-chain record.

### 3. Wallet Attestation

**PDA seed**: `[b"wallet", sha256(file_bytes), wallet_pubkey.as_ref()]`

Links a Solana wallet to the file. One per (file, wallet) pair.

| Field | Example | What it means |
|-------|---------|---------------|
| `content_hash` | `a1b2c3...` | Same SHA-256 hash. |
| `wallet` | Solana pubkey | The wallet that signed the attestation message. |
| `proof_type` | `"wallet_signature"` | Always this value. |
| `submitted_by` | Solana pubkey | R3L authority. |
| `timestamp` | Unix timestamp | When recorded. |

**Who creates it**: The `/api/wallet/attest` endpoint (web UI) or automatically during `/api/edge/attest` (if the edge node registered with a wallet). The wallet owner signs the message `"R3L: attest <content_hash>"` with their private key. The server verifies the Ed25519 signature off-chain using PyNaCl, then the R3L authority submits the on-chain record.

**Current limitation**: The on-chain PDA is created by the R3L server's keypair, not the user's wallet. This means on-chain data proves "R3L verified this wallet signed a message" — not independently verifiable without trusting R3L. The wallet signature itself is not stored on-chain.

---

## Proof Modes

There are two ways to create a C2PA attestation:

### Trusted Verifier (default)
- The R3L server runs the verifier, extracts C2PA data, and submits the transaction using its authority keypair.
- `proof_type = "trusted_verifier"`
- Fast. Requires trusting the R3L server.

### ZK Proof (SP1 Groth16)
- The R3L server runs the SP1 zkVM prover, which re-executes the C2PA verification inside the VM and produces a Groth16 proof.
- The Solana program verifies the proof on-chain before writing the attestation.
- `proof_type = "zk_groth16"`
- Slow (requires GPU). Trustless — anyone can verify the proof.
- Uses SP1 SDK v5.x with the `sp1-solana` verifier.

---

## Services

### Verifier (`services/verifier/`)
Rust CLI binary. Takes a file path, extracts and validates C2PA metadata against trust lists, outputs JSON matching the `VerifyOutput` struct. Core fields: `content_hash`, `has_c2pa`, `trust_list_match`, `validation_state`, certificate info, digital source type.

Trust lists are stored in `data/trust/` (C2PA official + R3L curated).

### Prover (`services/prover/`)
SP1 zkVM project (guest + host). The guest program re-runs verification logic inside the zkVM. The host generates a Groth16 proof that can be verified on-chain. Only used when `proof_type = "zk_groth16"`.

### Solana Program (`services/provenance_attestation/`)
Anchor program with four instructions:
- `submit_proof` — verifies SP1 Groth16 proof on-chain, creates `Attestation` PDA
- `submit_attestation` — authority-gated, creates `Attestation` PDA (trusted verifier mode)
- `submit_identity` — authority-gated, creates `IdentityAttestation` PDA
- `submit_wallet_identity` — authority-gated, creates `WalletAttestation` PDA

All instructions except `submit_proof` require the caller to be the hardcoded `AUTHORITY` pubkey.

### API (`services/api-py/`)
Python FastAPI server. Orchestrates everything:

| Endpoint | Method | What it does |
|----------|--------|-------------|
| `/api/verify` | POST | Upload file → run verifier binary → return `VerifyOutput` JSON |
| `/api/attest` | POST | Upload file → verify → submit C2PA attestation to Solana |
| `/api/prove` | POST | Upload file → verify → run SP1 prover → return proof |
| `/api/submit` | POST | Take proof + public inputs → submit to Solana (on-chain ZK verification) |
| `/api/verify-email/start` | POST | Upload file + email → send verification email with link |
| `/api/verify-email/status/:token` | GET | Check if email link was clicked |
| `/api/verify-email/attest` | POST | Submit identity attestation after email verified |
| `/api/wallet/attest` | POST | Verify Ed25519 signature → submit wallet attestation to Solana |
| `/api/attestation/:hash` | GET | Lookup attestation by content hash (DB first, then on-chain fallback) |
| `/api/attestations` | GET | List all attestations |
| `/api/edge/register` | POST | Verify wallet signature → create API key tied to wallet |
| `/api/edge/attest` | POST | API-key-gated → submit C2PA + auto wallet attestation |
| `/api/health` | GET | Returns "ok" |

**Data stores**:
- PostgreSQL — attestation records, customer/API-key table, email verification tokens
- Solana — on-chain PDAs (source of truth)
- The DB is a read cache. Lookup falls back to on-chain deserialization if the DB misses.

### Web Frontend (`services/web/`)
Vue 3 SPA with four pages:
- **Verify & Attest** — Upload file → see verification report card → optionally add email/wallet identity → attest on Solana
- **Lookup** — Search by content hash → see all attestation signals
- **Identity** — Standalone email verification flow
- **Home** — Landing page describing the verification signals

---

## Edge Nodes

Edge nodes are external operators that run the verifier locally and submit results via API key.

### Registration
1. Generate a Solana keypair
2. Sign `"R3L: register"` with the private key
3. `POST /api/edge/register` with `{pubkey, message, signature}`
4. Server verifies Ed25519 signature (off-chain, no Solana tx), creates a `Customer` row, returns a UUID API key

### Attestation
1. Run the verifier binary locally on a file
2. `POST /api/edge/attest` with the verification results + `X-API-Key` header
3. Server submits C2PA attestation to Solana
4. If the API key is linked to a wallet, server auto-creates a wallet attestation PDA

See [docs/run_edge_node.md](run_edge_node.md) for setup instructions.

---

## Lookup

`GET /api/attestation/:content_hash` returns the merged attestation record. The lookup flow:

1. Check PostgreSQL for a cached row
2. If not found, derive the PDA `[b"attestation", content_hash_bytes]` and fetch from Solana RPC
3. Deserialize the on-chain Borsh data into the canonical format

`GET /api/attestations` lists all known attestations (from DB), showing the `kind` field (`c2pa`, `identity`, `wallet`) and relevant metadata per kind.

Currently lookup is **exact match only** — you need the full SHA-256 content hash. There is no fuzzy search, no similarity search, no reverse image lookup.

---

## Embeddings & Vector Similarity

**Not implemented.** The system is entirely content-hash based (SHA-256 exact match). There is:

- No embedding model (CLIP, etc.)
- No vector database (Pinecone, pgvector, etc.)
- No similarity search or reverse image lookup
- No perceptual hashing

This means if a file is re-encoded, cropped, or screenshots are taken, it gets a different SHA-256 hash and cannot be linked back to existing attestations. Embedding-based similarity would address this gap.

---

## Deployment Tiers

R3L is designed around three deployment tiers, each targeting a different use case and trust model:

### Tier 1: Web GUI

**Status**: Implemented.

The hosted web application at `services/web/`. Users upload files through a browser, the server handles verification and attestation. Best for casual verification — checking files you received, one-off attestations.

- **Trust model**: User trusts the R3L server to run the verifier honestly and submit correct attestations.
- **Target users**: Journalists checking sources, consumers verifying media, anyone who wants to look up a file's provenance.
- **Tradeoff**: Files are uploaded to the R3L server. Fast and easy, but not suitable for sensitive media or high-volume workflows.

### Tier 2: Edge Node

**Status**: API endpoints implemented. No standalone client binary yet.

External operators run the R3L verifier binary locally and submit pre-verified results via API key. Files never leave the operator's network — only metadata (content hash, C2PA signals) goes to the cloud.

- **Trust model**: The edge node operator is accountable via their wallet-linked API key. R3L trusts the operator's verification results. Third parties trust R3L's on-chain record plus the wallet identity of the submitter.
- **Target users**: Newsrooms, stock photo agencies, social media platforms, content moderation pipelines — any organization processing media at volume.
- **Tradeoff**: Requires running the verifier binary locally. Results are only as trustworthy as the edge node operator.
- **What's needed**: A standalone CLI client or Python package that wraps the verifier binary + API calls into a single `r3l verify-and-attest <file>` command. Currently requires manual curl.

### Tier 3: iPhone App (Capture + Attest)

**Status**: Not implemented.

A native iOS app that captures photos with C2PA metadata and immediately attests them to R3L. This is the "proof at the point of capture" story — the strongest provenance chain possible.

- **Trust model**: Hardware-backed. iPhones (15 Pro+) embed C2PA metadata signed by Apple's certificate at capture time. The app would add R3L's identity layer (wallet attestation) and on-chain record on top.
- **Target users**: Photojournalists, citizen journalists, anyone who needs to prove "I took this photo, at this time, with this device."
- **What it provides that the other tiers don't**:
  - C2PA signed at capture (Apple's hardware-backed cert) — not just verified after the fact
  - Wallet identity bound at the moment of capture
  - On-chain attestation created immediately (or queued for offline sync)
  - Full chain: hardware signing → C2PA metadata → wallet identity → on-chain record
- **Considerations**:
  - Apple controls the C2PA signing certificate — you cannot use your own. R3L's value is the identity layer and on-chain record, not the C2PA signing itself.
  - Must handle offline capture gracefully. The content hash is deterministic (SHA-256 of file bytes), so the app can capture offline and attest later when connectivity returns. The attestation will reference the same hash regardless of when it's submitted.
  - App Store review: Apple may scrutinize blockchain/crypto functionality. The wallet signing UX needs to be clean.

### How the Tiers Complement Each Other

| | Web GUI | Edge Node | iPhone App |
|---|---------|-----------|------------|
| **Who verifies** | R3L server | Edge operator (local) | Apple hardware + R3L |
| **File leaves device** | Yes (uploaded) | No (only metadata sent) | No (attested on-device) |
| **C2PA source** | Extracted from uploaded file | Extracted locally | Created at capture |
| **Identity binding** | Email + wallet (optional) | Wallet (via API key) | Wallet (on-device) |
| **Volume** | One-off | Batch/pipeline | Per-capture |
| **Trust anchor** | R3L server | Edge operator's wallet | Apple cert + wallet |

The tiers are not mutually exclusive. A photo captured by the iPhone app can later be looked up through the web GUI. An edge node can process the same files that were originally attested through the web. The content hash is the universal key that ties everything together.

---

## Open Gaps

### Embeddings & Similarity Search
The biggest architectural gap across all three tiers. Currently the system is **SHA-256 exact match only**. If a file is re-encoded, cropped, screenshotted, or resized, it gets a different hash and cannot be linked to existing attestations.

This matters most for the iPhone app tier: a journalist captures a photo, it gets attested, then someone screenshots it and posts it on social media. Without similarity search, the link to the original attestation is lost.

Potential approach:
- Generate a visual embedding (CLIP or similar) at verification time
- Store embeddings in pgvector alongside the content hash
- Add a `POST /api/search` endpoint that accepts an image and returns the nearest attested files by cosine similarity
- The embedding becomes a secondary index — content hash remains the primary key for on-chain records

### Wallet Verification Strength
Registration verifies wallet ownership via Ed25519 signature (off-chain). But:
- The on-chain PDA is signed by the R3L server, not the user's wallet
- Per-attestation wallet signatures are not required (API key is trusted after registration)
- No check that the wallet exists on-chain as a funded Solana account

For stronger guarantees, options include: requiring per-attestation signatures, storing the user's signature on-chain, or having the user co-sign the Solana transaction.

### Edge Node Client
The edge node API exists but there's no packaged client. Operators must manually run the verifier binary and construct curl requests. A CLI tool or Python package would make this tier actually usable.

---

## Infrastructure

### Local Development
`./dev.sh` starts:
1. PostgreSQL (Docker container `r3l-postgres`)
2. Solana test validator (with program pre-loaded via `--bpf-program`)
3. Python API server (uvicorn with `--reload`)

### Production
- Docker Compose with API container + Postgres
- Solana program deployed to devnet/mainnet
- GPU instance (g5.2xlarge) required for ZK proving
- The `AUTHORITY` constant in the Solana program must match the server's keypair
