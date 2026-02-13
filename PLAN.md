# Trustless C2PA Provenance Attestations on Solana

## Context

The verifier CLI reads C2PA manifests from media files and outputs structured JSON with trust validation. We want to put these attestation records on the Solana blockchain so anyone can look up a file's provenance by its content hash.

The system is **trustless** — no one submits attestation data directly. Instead, an SP1 zkVM prover runs the C2PA verification off-chain and produces a Groth16 proof. The on-chain program verifies the proof mathematically before storing the attestation. Anyone can submit, and the math guarantees correctness.

### Why ZK, not on-chain C2PA?

C2PA verification requires ECDSA P-256 signature checks (no Solana precompile), X.509 cert chain parsing, JUMBF/CBOR decoding, and trust anchor matching — far exceeding Solana's 1.4M compute unit limit. A Groth16 proof compresses all of that into 3 BN254 pairing checks (~280K CU) using Solana's native `alt_bn128` precompiles.

## Full Vision

Three independent services, each usable on its own:

```
  ┌──────────────────┐    ┌──────────────────┐    ┌────────────────────────────┐
  │  verifier        │    │  prover          │    │  provenance_attestation    │
  │  (Rust crate)    │    │  (SP1 project)   │    │  (Anchor program)         │
  │                  │    │                  │    │                            │
  │  verify()        │    │  host: extracts  │    │  submit_proof()            │
  │  → VerifyOutput  │───▶│  crypto evidence │───▶│  → verifies Groth16 proof  │
  │  + content_hash  │    │  guest: re-      │    │  → stores Attestation PDA  │
  │                  │    │  verifies in     │    │                            │
  │  Standalone:     │    │  zkVM            │    │  Lookup:                   │
  │  just check a    │    │  → Groth16 proof │    │  derive PDA from           │
  │  file's C2PA     │    │  + public outputs│    │  content_hash, read it     │
  └──────────────────┘    └──────────────────┘    └────────────────────────────┘
        ▲                       ▲                          ▲
        │                       │                          │
  ┌─────┴──────────────────────┴──────────────────────────┴─────┐
  │                        Entry Points                          │
  │                                                              │
  │  CLI: each step is a separate command                        │
  │    verifier /path/to/image.png          (just verify)        │
  │    prove --file output.json             (generate proof)     │
  │    submit --proof proof.json            (submit to Solana)   │
  │    query --hash <content_hash>          (lookup on-chain)    │
  │                                                              │
  │  Web API (Phase 2): each step is a separate endpoint         │
  │    POST /api/verify     → VerifyOutput                       │
  │    POST /api/prove      → proof                              │
  │    POST /api/submit     → tx_signature                       │
  │    GET  /api/attestation/:hash → stored attestation          │
  │                                                              │
  │  Web UI (Phase 2): each step is a separate button            │
  │    [Upload] → [Verify] → [Prove] → [Submit to Solana]       │
  │    [Lookup by hash]                                          │
  └──────────────────────────────────────────────────────────────┘
```

**Key design principle:** Each service is independent. A user who just wants to check a file's C2PA metadata never touches the prover or Solana. A user who already has a proof can submit it directly. The web API and UI orchestrate the steps but each step is a separate call.

Two phases:

1. **Phase 1** — CLI end-to-end: verifier lib + content_hash, SP1 prover, Solana program, submit CLI
2. **Phase 2** — Web: API server + browser UI for file upload, verification, proof generation, Solana lookup

---

## Phase 1: CLI End-to-End (Trustless)

```
  verifier                    prover                   provenance_attestation
  ────────                    ──────                   ──────────────────────
  cargo run --bin verifier    cargo run --bin prove    submit --proof proof.json
  -- /path/to/image.png       -- --file output.json
        │                          │                          │
        ▼                          ▼                          ▼
  VerifyOutput JSON ──────▶  Groth16 proof  ──────▶  Attestation PDA on Solana
  + content_hash             + public outputs
```

### Step 1: Refactor verifier into lib + binary

**Files:**
- `services/verifier/src/lib.rs` — extract all logic from main.rs
- `services/verifier/src/main.rs` — thin CLI wrapper
- `services/verifier/Cargo.toml` — add `sha2 = "0.10"`

Changes:
- Move `VerifyOutput`, `Props`, `load_pems()`, `try_read()`, `is_trusted()`, `resolve_trust()`, `extract_cn()`, `extract_props()` into `lib.rs`
- Add public `verify(path: &str, trust_dir: &str) -> Result<VerifyOutput>` function
- Add `content_hash: Option<String>` to `VerifyOutput` (hex-encoded SHA-256 of file bytes)
- `main.rs` becomes ~15 lines: parse args, call `verifier::verify()`, print JSON
- The verifier is fully standalone — no dependency on prover or Solana

### Step 2: Create SP1 prover

**New service:** `services/prover/`

The challenge: c2pa-rs cannot compile inside SP1's RISC-V zkVM (depends on `ring`, `openssl`, networking). So the host extracts raw cryptographic evidence and the guest re-verifies just the crypto primitives.

```
services/prover/
├── program/                    # SP1 guest (compiles to RISC-V)
│   ├── Cargo.toml              # sp1-zkvm, sha2, p256, x509-cert
│   └── src/
│       └── main.rs             # ZK verification logic
├── script/                     # SP1 host (runs natively)
│   ├── Cargo.toml              # sp1-sdk, c2pa, verifier (path dep)
│   ├── build.rs                # compiles guest ELF
│   └── src/
│       └── bin/
│           ├── prove.rs        # extract evidence → run guest → Groth16 proof
│           └── vkey.rs         # output verification key hash
└── shared/                     # types shared between host + guest
    ├── Cargo.toml              # serde, borsh
    └── src/
        └── lib.rs              # CryptoEvidence, PublicOutputs structs
```

**Host program** (runs natively):
1. Reads verifier JSON output (from file or stdin)
2. Extracts raw cryptographic evidence from the C2PA manifest:
   - COSE_Sign1 envelope bytes (signature + signed payload)
   - X.509 certificate chain (DER-encoded leaf + intermediates + root)
   - Claim hash bytes
   - Trust anchor PEMs
3. Reads file bytes for SHA-256 verification
4. Feeds all of this as private inputs to the SP1 guest
5. Collects the Groth16 proof + public outputs

**Guest program** (runs inside zkVM, proves correctness):
1. Reads private inputs: COSE signature, cert chain, claim bytes, trust anchors, file bytes
2. Verifies:
   - SHA-256(file bytes) = content_hash
   - COSE signature is valid over claim payload (using `p256` patched crate)
   - Certificate chain is valid (each cert signed by parent, root is in trust anchors)
3. Commits **public outputs**: content_hash, has_c2pa, trust_list_match, validation_state, issuer, digital_source_type, common_name, software_agent, signing_time

**Shared types** (`CryptoEvidence` for inputs, `PublicOutputs` for committed outputs)

**SP1 crypto patches** (in program/Cargo.toml):
```toml
[patch.crates-io]
sha2 = { git = "https://github.com/sp1-patches/RustCrypto-hashes", tag = "patch-sha2-0.10.9-sp1-4.0.0" }
p256 = { git = "https://github.com/sp1-patches/elliptic-curves", tag = "patch-p256-13.2-sp1-5.0.0" }
```

### Step 3: Create provenance_attestation program with Anchor

**New service:** `services/provenance_attestation/`

```
services/provenance_attestation/
├── Anchor.toml
├── Cargo.toml                  # workspace
├── programs/
│   └── provenance_attestation/
│       ├── Cargo.toml          # anchor-lang, sp1-solana, borsh
│       └── src/
│           ├── lib.rs          # declare_id!, #[program] with submit_proof
│           ├── state.rs        # Attestation account struct
│           ├── errors.rs       # custom error codes
│           └── constants.rs    # seeds, SP1 vkey hash
├── tests/
│   └── provenance_attestation.ts           # integration tests
└── package.json                # @coral-xyz/anchor, mocha
```

**Dependencies:**
- `anchor-lang = "0.32"`
- `sp1-solana = { git = "https://github.com/succinctlabs/sp1-solana" }`

**On-chain account (PDA per file):**
```rust
#[account]
pub struct Attestation {
    pub content_hash: [u8; 32],          // SHA-256 of file (PDA seed)
    pub has_c2pa: bool,
    pub trust_list_match: String,        // "official" | "curated" | "untrusted"
    pub validation_state: String,        // "Trusted" | "Valid" | "Invalid"
    pub digital_source_type: String,     // IPTC source type or empty
    pub issuer: String,                  // cert issuer org
    pub common_name: String,             // cert CN
    pub software_agent: String,          // content creation tool
    pub signing_time: String,            // ISO timestamp
    pub submitted_by: Pubkey,            // who submitted the tx
    pub timestamp: i64,                  // Solana clock timestamp
    pub bump: u8,
}
```

**Instruction:**

`submit_proof` — anyone submits a ZK proof to store an attestation:
- Takes Groth16 proof bytes + public outputs as arguments
- Calls `sp1_solana::verify_proof()` with the hardcoded verification key hash
- If proof is valid, decodes public outputs into attestation fields
- Creates PDA seeded by `[b"attestation", content_hash]`
- Stores all fields + sets `timestamp = Clock::get()`
- Fails if PDA already exists (one attestation per file)
- **No authority check** — the proof IS the authorization
- Requires compute budget increase (~280K CU for proof verification)

`get_attestation` — not an instruction, just a PDA derivation:
- Client derives PDA from `[b"attestation", content_hash]` and reads the account
- No on-chain instruction needed for reads

### Step 4: Create submit CLI

**New file:** `scripts/submit-attestation.ts`

TypeScript script using `@coral-xyz/anchor` that:
1. Accepts `--proof` with a proof JSON file (output from prover)
2. Derives the PDA from content_hash in the proof's public outputs
3. Builds and sends `submit_proof` transaction with proof + public outputs
4. Prints the transaction signature and PDA address

```
Usage:
  # Full pipeline: verify → prove → submit (each step independent)
  cargo run --bin verifier -- /path/to/image.png > output.json
  cargo run --bin prove -- --file output.json > proof.json
  npx ts-node scripts/submit-attestation.ts --proof proof.json

  # Just verify (no Solana, no ZK):
  cargo run --bin verifier -- /path/to/image.png

  # Query an existing attestation:
  npx ts-node scripts/submit-attestation.ts --query <content_hash_hex>
```

**Config:** reads Solana cluster + wallet from environment or Anchor.toml:
- `ANCHOR_PROVIDER_URL` (default: `http://localhost:8899` for localnet)
- `ANCHOR_WALLET` (default: `~/.config/solana/id.json`)

### Step 5: Integration test on localnet

1. Start `solana-test-validator`
2. `anchor build && anchor deploy` the provenance_attestation program
3. Run verifier on `data/samples/chatgpt.png` → capture JSON
4. Run SP1 prover → generate Groth16 proof
5. Run submit CLI → submits proof to localnet
6. Query the PDA by content hash → verify all fields match original verifier output

### Key Constraints

- **Solana account size**: Strings in Anchor are stored as `4 bytes (length) + utf8 bytes`. Cap string fields at 64 bytes each. Total account size ~500 bytes.
- **Transaction size**: Groth16 proof (256 bytes) + public inputs + instruction data fits within Solana's 1,232-byte limit, but string fields in public outputs need bounded sizes.
- **Compute units**: SP1 Groth16 verification costs ~280K CU, above Solana's default 200K limit. Must request compute budget increase in the transaction.
- **Proving time**: Groth16 proofs for complex programs take minutes. Fine for an attestation service, not for real-time use.
- **SP1 guest constraints**: c2pa-rs can't compile in zkVM. Host extracts raw crypto evidence, guest re-verifies only the cryptographic primitives (P-256 signatures, cert chain, SHA-256 hashes).

### Verification

1. `cargo run -- ../../data/samples/chatgpt.png` → JSON includes `content_hash` field
2. SP1 prover generates valid Groth16 proof from verifier output
3. `anchor test` on localnet → submit_proof verifies proof and stores attestation, PDA is readable
4. End-to-end: verifier → prover → submit → query → fields match

---

## Phase 2: Web API + UI

The web API orchestrates the same three independent services. Each step is a separate endpoint — the UI calls them in sequence, but the user controls when to proceed.

```
  ┌───────────┐
  │  Web UI   │  React app
  │  :3000    │
  └─────┬─────┘
        │ fetch()
        ▼
  ┌───────────┐     ┌──────────┐  ┌──────────┐  ┌──────────────────────────┐
  │  Web API  │────▶│ verifier │  │  prover  │  │  provenance_attestation  │
  │  axum     │     │  crate   │  │  (SP1)   │  │  (Anchor on Solana)      │
  │  :8080    │     └──────────┘  └──────────┘  └──────────────────────────┘
  └───────────┘
```

### Web API (`services/api/`)

```
services/api/
├── Cargo.toml          # axum, tokio, verifier (path dep), solana-sdk, solana-client, sp1-sdk
└── src/
    └── main.rs
```

**Endpoints:**
```
POST /api/verify              file upload → verifier::verify() → VerifyOutput JSON
POST /api/prove               VerifyOutput JSON → SP1 prover → proof JSON
POST /api/submit              proof JSON → submit to Solana → { tx_signature, pda_address }
GET  /api/attestation/:hash   read PDA from Solana → stored Attestation JSON
GET  /api/health              { ok: true }
```

Each endpoint is independent. A client can call `/api/verify` and stop there. Or call `/api/prove` with previously saved verifier output. Or call `/api/submit` with a pre-generated proof.

**Config:** `TRUST_DIR`, `SOLANA_RPC_URL`, `PORT`

### Web UI (`services/web/`)

```
services/web/
├── package.json        # react, vite
├── index.html
├── src/
│   ├── main.tsx
│   ├── App.tsx
│   ├── components/
│   │   ├── FileUpload.tsx      # drag-and-drop zone
│   │   ├── ResultCard.tsx      # displays VerifyOutput fields
│   │   ├── ProofStatus.tsx     # shows proving progress
│   │   └── LookupForm.tsx      # search by content hash
│   └── api.ts                  # fetch wrappers for /api/*
└── vite.config.ts              # proxy /api to :8080
```

**Flow:**
1. **Upload** — drag-and-drop file → calls `/api/verify` → show C2PA results
2. **Prove** — click "Generate Proof" → calls `/api/prove` → show progress → proof ready
3. **Submit** — click "Submit to Solana" → calls `/api/submit` → show tx signature
4. **Lookup** — search by content hash → calls `/api/attestation/:hash` → show stored attestation or "not found"

User can stop at any step. Each button only appears after the previous step completes.

### Docker Compose

```yaml
services:
  api:
    build: { context: ., dockerfile: docker/api.Dockerfile }
    ports: ["8080:8080"]
    environment:
      TRUST_DIR: /data/trust
      SOLANA_RPC_URL: http://localhost:8899
    volumes: [./data:/data:ro]

  web:
    build: { context: ., dockerfile: docker/web.Dockerfile }
    ports: ["3000:3000"]
```
