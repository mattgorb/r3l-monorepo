# Architecture: Trustless C2PA Attestations on Solana

## What This Does

This system takes a media file (image, video, audio), reads its [C2PA](https://c2pa.org/) provenance metadata (who made it, what tool, whether it's AI-generated, whether the signature is trusted), and stores that attestation on the Solana blockchain — so anyone can look up a file's provenance by its SHA-256 hash.

The system is **trustless**. Nobody submits attestation data directly. Instead, a ZK prover runs the C2PA verification off-chain and produces a Groth16 proof. The on-chain program verifies the proof mathematically before storing anything. Anyone can submit, and the math guarantees the data is correct.

## Why ZK Instead of On-Chain Verification

C2PA verification requires ECDSA P-256 signature checks, X.509 certificate chain parsing, JUMBF/CBOR decoding, and trust anchor matching. This would cost ~2-4M compute units on Solana, far exceeding the 1.4M CU limit per transaction. There's also no P-256 precompile on Solana.

A Groth16 proof compresses all of that work into 3 BN254 pairing checks (~280K CU) using Solana's native `alt_bn128` precompiles. The proof is ~256 bytes and fits easily in a transaction.

## Three Independent Services

Each service is standalone. You can verify a file without proving or submitting. You can prove without submitting. You can submit a pre-generated proof without running the verifier.

```
  ┌──────────────────┐    ┌──────────────────┐    ┌────────────────────────────┐
  │  verifier        │    │  prover          │    │  provenance_attestation    │
  │  Rust crate      │    │  SP1 project     │    │  Anchor program (Solana)   │
  │                  │    │                  │    │                            │
  │  Input: file     │    │  Input: verifier │    │  Input: Groth16 proof      │
  │  Output: JSON    │    │  JSON + file     │    │  Output: PDA on-chain      │
  │  with C2PA data  │───>│  Output: Groth16 │───>│  with attestation data     │
  │  + content_hash  │    │  proof + public  │    │                            │
  │                  │    │  outputs         │    │  Anyone can read it by     │
  │                  │    │                  │    │  deriving PDA from hash    │
  └──────────────────┘    └──────────────────┘    └────────────────────────────┘
```

---

## Service 1: Verifier (`services/verifier/`)

**What it does:** Reads a media file's C2PA manifest using the `c2pa-rs` SDK and produces structured JSON.

**Key files:**
- `src/lib.rs` — all verification logic, public `verify()` and `verify_with_env()` functions
- `src/main.rs` — thin CLI wrapper

**How it works:**
1. Reads the file bytes and computes `SHA-256(file_bytes)` → `content_hash`
2. Calls `c2pa::Reader::from_file()` to parse the C2PA manifest
3. Extracts the active manifest's claim, signature info, and validation status
4. Matches the signing certificate against trust lists (official/curated/untrusted)
5. Outputs `VerifyOutput` JSON with all fields

**Usage:**
```bash
cargo run --bin verifier -- /path/to/image.png
```

**Output (abbreviated):**
```json
{
  "path": "chatgpt.png",
  "content_hash": "4ebd98d3893a16a6b3cf73c4b3cdf3b55149af563c47e07dd58f9bba17a8aabf",
  "has_c2pa": true,
  "trust_list_match": "untrusted",
  "validation_state": "Invalid",
  "digital_source_type": "http://cv.iptc.org/.../trainedAlgorithmicMedia",
  "software_agent": "GPT-4o",
  "issuer": "OpenAI",
  "common_name": "Truepic Lens CLI in Sora"
}
```

The `content_hash` is the lookup key for everything downstream. It uniquely identifies the file.

---

## Service 2: Prover (`services/prover/`)

**What it does:** Takes the verifier's JSON output + the original file, runs a ZK program inside SP1's RISC-V virtual machine, and produces a Groth16 proof that the verification was done correctly.

**Why the split architecture:** The `c2pa-rs` crate depends on `ring`, OpenSSL, and networking — none of which compile inside a zkVM. So the work is split:

```
  Host (runs natively)              Guest (runs inside zkVM)
  ─────────────────────             ────────────────────────
  1. Run c2pa-rs verifier           1. Read private inputs
  2. Extract raw crypto evidence:   2. Re-verify cryptographic primitives:
     - COSE_Sign1 signature            - SHA-256(file) = content_hash ✓
     - X.509 cert chain (DER)          - COSE signature valid (TODO)
     - Claim payload bytes             - Cert chain valid (TODO)
     - Trust anchor certs           3. Commit public outputs
  3. Feed to guest as private       4. SP1 generates Groth16 proof
     inputs via SP1Stdin
```

**Project structure:**
```
services/prover/
├── program/          # Guest (compiles to RISC-V ELF, runs inside zkVM)
│   └── src/main.rs   # ZK verification logic
├── script/           # Host (runs natively)
│   └── src/bin/
│       ├── prove.rs  # Full pipeline: extract → execute guest → Groth16 proof
│       └── vkey.rs   # Print the verification key hash
└── shared/           # Types shared between host and guest
    └── src/lib.rs    # CryptoEvidence (private inputs), PublicOutputs (committed)
```

**Data flow through the prover:**

```
Private inputs (only the prover sees these):
┌─────────────────────────────────────────────┐
│ CryptoEvidence {                            │
│   file_bytes: [raw file content]            │
│   cose_sign1: [COSE signature envelope]     │  ← TODO: extract from manifest
│   cert_chain_der: [[leaf], [intermediate]]  │  ← TODO: extract from manifest
│   claim_bytes: [signed payload]             │  ← TODO: extract from manifest
│   trust_anchors_der: [[root certs]]         │  ← TODO: load from trust dir
│ }                                           │
│ + attestation fields from verifier JSON     │
└─────────────────────────────────────────────┘
                    │
                    ▼  (fed to SP1 guest via SP1Stdin)
┌─────────────────────────────────────────────┐
│ Guest program (RISC-V zkVM):                │
│   1. SHA-256(file_bytes) → content_hash     │  ← proven correct
│   2. Verify COSE signature (TODO)           │
│   3. Verify cert chain (TODO)               │
│   4. Commit PublicOutputs                   │
└─────────────────────────────────────────────┘
                    │
                    ▼
Public outputs (go on-chain, anyone can read):
┌─────────────────────────────────────────────┐
│ PublicOutputs {                              │
│   content_hash: [32 bytes],                 │
│   has_c2pa: true,                           │
│   trust_list_match: "untrusted",            │
│   validation_state: "Invalid",              │
│   issuer: "OpenAI",                         │
│   ...                                       │
│ }                                           │
│ + Groth16 proof (~256 bytes)                │
└─────────────────────────────────────────────┘
```

**Usage:**
```bash
# Generate proof (mock mode for testing — instant, no real crypto):
cargo run --bin prove -- --file output.json --media image.png --mock

# Generate real Groth16 proof (takes minutes, needs CPU):
cargo run --bin prove -- --file output.json --media image.png

# Print verification key hash (needed by on-chain program):
cargo run --bin vkey
# → 0x0014beac9f9d3c39486f2537e8e5aa7ec0efbf648b9b5ef7e1b403c1d6dc4a1a
```

**Current status:** The guest currently only proves SHA-256 content hash correctness. COSE signature verification and X.509 cert chain verification are TODO — they need pure-Rust implementations (`p256`, `x509-cert` crates) with SP1 patches for accelerated execution inside the zkVM.

---

## Service 3: Provenance Attestation (`services/provenance_attestation/`)

**What it does:** Solana program (smart contract) that stores attestation records. It has one instruction: `submit_proof`, which verifies a Groth16 proof and stores the attestation in a PDA (Program Derived Address).

**Key files:**
- `programs/provenance_attestation/src/lib.rs` — `submit_proof` instruction
- `programs/provenance_attestation/src/state.rs` — `Attestation` account struct
- `programs/provenance_attestation/src/constants.rs` — PDA seed, SP1 vkey hash
- `programs/provenance_attestation/src/errors.rs` — custom error codes

**On-chain account (one per file):**

```rust
#[account]
pub struct Attestation {
    pub content_hash: [u8; 32],       // SHA-256 of file — PDA seed
    pub has_c2pa: bool,
    pub trust_list_match: String,     // "official" | "curated" | "untrusted"
    pub validation_state: String,     // "Trusted" | "Valid" | "Invalid"
    pub digital_source_type: String,  // IPTC source type URI
    pub issuer: String,               // cert issuer organization
    pub common_name: String,          // cert common name
    pub software_agent: String,       // content creation tool
    pub signing_time: String,         // ISO timestamp
    pub submitted_by: Pubkey,         // who submitted the tx
    pub timestamp: i64,               // Solana clock timestamp
    pub bump: u8,                     // PDA bump seed
}
```

**How `submit_proof` works:**

```
Transaction arrives with:
  - proof bytes (Groth16, ~256 bytes)
  - public_inputs bytes
  - content_hash + attestation fields (decoded from proof's public outputs)

  1. Verify Groth16 proof (TODO — sp1_solana::verify_proof())
     Uses hardcoded verification key hash to check proof
     ~280K compute units via alt_bn128 precompiles

  2. Validate string lengths (each ≤ 128 bytes)

  3. Create PDA: seeds = ["attestation", content_hash]
     If PDA already exists → transaction fails (one attestation per file)

  4. Store all fields in the PDA account

  No authority check — the proof IS the authorization.
  Anyone with a valid proof can submit.
```

**How to read an attestation (no instruction needed):**

```typescript
// Derive PDA from content hash
const [pda] = PublicKey.findProgramAddressSync(
  [Buffer.from("attestation"), contentHashBytes],
  programId
);
// Read account data directly
const attestation = await program.account.attestation.fetch(pda);
```

**Program ID:** `HahVgC9uo73aLw1ouBEvgMT7KmGTS6rovfbKP9zuCtjc`

---

## Service 4: Web API (`services/api/`)

**What it does:** HTTP API that orchestrates the three services above. Upload a file, verify it, generate a proof, submit to Solana, and look up attestations — all from the browser or any HTTP client.

**Key files:**
- `src/main.rs` — axum router, CORS, static file serving, app state
- `src/routes/verify.rs` — `POST /api/verify`
- `src/routes/prove.rs` — `POST /api/prove`
- `src/routes/submit.rs` — `POST /api/submit`
- `src/routes/attestation.rs` — `GET /api/attestation/{hash}`

**Endpoints:**

```
POST /api/verify              multipart file upload → verifier::verify() → VerifyOutput JSON
POST /api/prove               multipart file upload → shell out to SP1 prover → proof + public outputs
POST /api/submit              JSON body → build Solana tx → submit to RPC → { signature, pda }
GET  /api/attestation/{hash}  hex content hash → fetch PDA from Solana → Attestation JSON (or 404)
GET  /api/health              "ok"
```

**How it integrates the services:**

| Endpoint | Integration method | Why |
|---|---|---|
| `/api/verify` | `verifier::verify()` as a Rust path dependency | Direct function call, no serialization overhead. Uses `spawn_blocking` because verifier is sync. |
| `/api/prove` | Shell out to `cargo run --bin prove` in `services/prover/` | SP1 SDK is too large to compile into the API binary. Prover writes a JSON sidecar with proof + public outputs. |
| `/api/submit` | Build raw Solana transaction with `solana-sdk v2` + `solana-rpc-client` | Manual Anchor instruction encoding (8-byte discriminator + borsh args). No Anchor TS SDK needed. |
| `/api/attestation` | Fetch account via `solana-rpc-client`, borsh-deserialize | Skip 8-byte Anchor discriminator, deserialize remaining bytes into Attestation fields. |

**Solana transaction construction (submit endpoint):**

The API manually constructs Anchor-compatible transactions without importing the Anchor framework:

```
Instruction data layout:
┌──────────────────┬─────────┬───────────────┬──────────────┬─────────┬─────────────┐
│ discriminator    │ proof   │ public_inputs │ content_hash │ has_c2pa│ strings...  │
│ [54,241,46,84,   │ Vec<u8> │ Vec<u8>       │ [u8; 32]     │ bool    │ 7x String   │
│  4,212,46,94]    │ borsh   │ borsh         │ raw          │ borsh   │ borsh       │
│ 8 bytes          │         │               │ 32 bytes     │ 1 byte  │             │
└──────────────────┴─────────┴───────────────┴──────────────┴─────────┴─────────────┘

Accounts:
  0. attestation PDA (writable)
  1. submitter/payer (writable, signer)
  2. system_program (readonly)
```

**Dependency note:** `solana-sdk v2` is required (not v1.18) because `c2pa-rs` needs `zeroize >= 1.8`, while `solana-sdk v1.18` depends on `curve25519-dalek v3` which caps `zeroize < 1.4`. The v2 SDK uses `curve25519-dalek v4` which is compatible.

**Static file serving:** In production, the API serves the built Vue app from `./static` using `tower-http::ServeDir`. Any path that doesn't match an `/api/*` route falls through to the SPA's `index.html`.

---

## Service 5: Web UI (`services/web/`)

**What it does:** Vue 3 single-page application that provides a browser interface for the full pipeline: upload, verify, prove, submit, and lookup.

**Stack:** Vue 3 + TypeScript + Vite + Tailwind CSS

**Key files:**
- `src/App.vue` — main layout, state management for verify result
- `src/api.ts` — axios client with calls to all API endpoints
- `src/types.ts` — TypeScript interfaces matching Rust structs
- `src/components/FileUpload.vue` — drag-and-drop file upload zone
- `src/components/Results.vue` — verification results display with trust badge
- `src/components/ProofStatus.vue` — ZK proof generation button + status
- `src/components/SubmitPanel.vue` — submit to Solana button + tx result
- `src/components/LookupForm.vue` — search attestations by content hash

**UI flow:**

```
┌─────────────────────────────────────────────────────────────┐
│  1. Upload                                                  │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  Drop a media file here (or click to browse)        │    │
│  └─────────────────────────────────────────────────────┘    │
│                          │                                  │
│                          ▼ POST /api/verify                 │
│  2. Results                                                 │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  [curated] C2PA metadata found                      │    │
│  │  Content Hash: 4ebd98d3...                          │    │
│  │  Issuer: OpenAI                                     │    │
│  │  Software Agent: GPT-4o                             │    │
│  │  Digital Source Type: trainedAlgorithmicMedia        │    │
│  │  ...                                                │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                             │
│  3. ZK Proof                                                │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  Generate a Groth16 proof    [Generate Proof]       │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                             │
│  4. Submit                                                  │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  Store attestation on-chain  [Submit to Solana]     │    │
│  │  Tx: 5xK2...                                       │    │
│  │  PDA: 7nR4...                                      │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                             │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│  5. Lookup                                                  │
│  ┌──────────────────────────────────┐                       │
│  │  Content hash (hex): [________] │  [Search]              │
│  └──────────────────────────────────┘                       │
│  → displays stored attestation or "not found"               │
└─────────────────────────────────────────────────────────────┘
```

**Trust badge colors:**
- Green: `official` (signed by trust list cert, e.g. Adobe, Microsoft)
- Yellow: `curated` (signed by project-curated cert, e.g. OpenAI via Truepic)
- Red: `untrusted` (has C2PA but signer not in any trust list)
- Gray: no C2PA metadata at all

**Dev proxy:** In development, Vite proxies `/api/*` requests to `http://localhost:3001` (the API server). In production, the API serves the built Vue app directly.

---

## Submit CLI (`services/provenance_attestation/scripts/submit.ts`)

TypeScript client that submits proofs and queries attestations from the command line (alternative to the web UI).

```bash
# Submit a proof:
npx ts-node scripts/submit.ts --proof proof.json

# Query by content hash:
npx ts-node scripts/submit.ts --query 4ebd98d3893a16a6b3cf73c4b3cdf3b55149af563c47e07dd58f9bba17a8aabf
```

---

## Full System Diagram

```
                     Browser
                        │
                        ▼
                ┌───────────────┐
                │  Vue SPA      │  services/web/
                │  :5173 (dev)  │  Tailwind + TypeScript
                └───────┬───────┘
                        │ /api/*
                        ▼
                ┌───────────────┐     ┌──────────────────┐
                │  axum API     │────▶│  verifier crate  │  (path dep, in-process)
                │  :3001        │     └──────────────────┘
                │               │
                │  services/    │     ┌──────────────────┐
                │  api/         │────▶│  prover binary   │  (shell out to cargo run)
                │               │     └──────────────────┘
                │               │
                │               │     ┌──────────────────┐
                │               │────▶│  Solana RPC      │  (solana-sdk v2)
                └───────────────┘     │  :8899           │
                                      └────────┬─────────┘
                                               │
                                      ┌────────▼─────────┐
                                      │  Solana program   │
                                      │  provenance_      │
                                      │  attestation      │
                                      │  (Anchor 0.30)    │
                                      └──────────────────┘
```

---

## What's Left (TODOs)

1. **COSE signature verification in guest** — Parse COSE_Sign1 envelope, verify ECDSA P-256 signature using `p256` crate with SP1 patches
2. **X.509 cert chain verification in guest** — Parse DER certs, verify each is signed by the next, root matches trust anchor
3. **Host crypto extraction** — Extract COSE, certs, claim bytes from C2PA manifest in the prover host
4. **Enable `sp1_solana::verify_proof()`** — Uncomment in lib.rs, add sp1-solana dep, test with real proofs
5. **Match proof public outputs to instruction args** — On-chain program should decode public outputs from proof and verify they match the submitted attestation fields
6. **Real Groth16 proofs** — Test with `--cpu` instead of `--mock` (takes minutes)
7. **Prover JSON sidecar** — Add `--json-out` flag to `prove.rs` so the API can read proof data without importing sp1-sdk

---

## Key Constants

| Constant | Value |
|---|---|
| Program ID | `HahVgC9uo73aLw1ouBEvgMT7KmGTS6rovfbKP9zuCtjc` |
| SP1 vkey hash | `0x0014beac9f9d3c39486f2537e8e5aa7ec0efbf648b9b5ef7e1b403c1d6dc4a1a` |
| PDA seed | `b"attestation"` + `content_hash` |
| Max string length | 128 bytes |
| Account space | 8 + 32 + 1 + 7*(4+128) + 32 + 8 + 1 = 1006 bytes |
| Proof verification CU | ~280,000 (request 400,000 budget) |
| API default port | 3001 |
| API body limit | 50 MB |
| Anchor instruction discriminator | `sha256("global:submit_proof")[..8]` = `[54,241,46,84,4,212,46,94]` |
| Anchor account discriminator | `sha256("account:Attestation")[..8]` = `[152,125,183,86,36,146,121,73]` |
