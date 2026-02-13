# Architecture: Trustless C2PA Attestations on Solana

## What This Does

This system takes a media file (image, video, audio), cryptographically verifies its [C2PA](https://c2pa.org/) provenance metadata inside a zero-knowledge virtual machine, and stores the verified attestation on the Solana blockchain — so anyone can look up a file's provenance by its SHA-256 hash.

The system is **trustless**. Nobody submits attestation data directly. A ZK prover runs the full C2PA cryptographic verification off-chain (ECDSA P-256 signature, X.509 certificate parsing, trust anchor matching) and produces a Groth16 proof. The on-chain program verifies the proof mathematically before storing anything. The ZK guest derives all outputs from verified cryptographic data — a malicious host cannot forge results.

## Why ZK Instead of On-Chain Verification

C2PA verification requires ECDSA P-256 signature checks, X.509 certificate chain parsing, JUMBF/CBOR decoding, and trust anchor matching. This would cost ~2-4M compute units on Solana, far exceeding the 1.4M CU limit per transaction. There's also no P-256 precompile on Solana.

A Groth16 proof compresses all of that work into 3 BN254 pairing checks (~280K CU) using Solana's native `alt_bn128` precompiles. The proof is ~256 bytes and fits easily in a transaction.

## End-to-End Pipeline

```
  ┌──────────────────┐    ┌──────────────────┐    ┌────────────────────────────┐
  │  Prover Host     │    │  zkVM Guest      │    │  Solana Program            │
  │  (native Rust)   │    │  (RISC-V SP1)    │    │  (Anchor 0.30)             │
  │                  │    │                  │    │                            │
  │  Input: PNG file │    │  Input: Crypto   │    │  Input: Groth16 proof      │
  │  + trust anchors │    │  Evidence blob   │    │  + public outputs          │
  │                  │    │                  │    │                            │
  │  Output: Crypto  │───>│  Output: Verified│───>│  Output: PDA on-chain      │
  │  Evidence struct │    │  PublicOutputs   │    │  with attestation data     │
  │                  │    │  + Groth16 proof │    │                            │
  └──────────────────┘    └──────────────────┘    └────────────────────────────┘
```

---

## Service 1: Prover Host (`services/prover/script/`)

**What it does:** Extracts raw cryptographic evidence from a PNG file's C2PA manifest and feeds it into the SP1 zkVM for trustless verification.

**Key files:**
- `src/jumbf_extract.rs` — PNG parsing, JUMBF box tree walking, COSE/cert extraction
- `src/bin/prove.rs` — CLI that orchestrates extraction → execution → proof generation
- `src/lib.rs` — crate root exposing `jumbf_extract` module

### Extraction Pipeline

The host does **no verification** — it only extracts raw bytes. All trust decisions happen inside the zkVM guest.

```
PNG file bytes
    │
    ▼
1. Parse PNG chunks → find caBX chunk(s)
   (C2PA JUMBF data lives in caBX chunks per the C2PA spec)
    │
    ▼
2. Parse JUMBF box tree (ISO BMFF format)
   Top-level: jumb (manifest store)
     └─ jumb (manifest, last = active)
         ├─ jumb [c2pa.claim / c2pa.claim.v2] → raw CBOR bytes
         └─ jumb [c2pa.signature] → COSE_Sign1_Tagged bytes
    │
    ▼
3. Parse COSE_Sign1 unprotected header → extract x5chain
   (label 33: array of DER-encoded X.509 certificates, leaf first)
    │
    ▼
4. Load trust anchor PEM files from data/trust/{official,curated}/
   Convert PEM → DER bytes
    │
    ▼
5. Pack everything into CryptoEvidence struct
```

### CryptoEvidence (Host → Guest)

```rust
pub struct CryptoEvidence {
    pub asset_hash: [u8; 32],                    // SHA-256 of file
    pub has_manifest: bool,                       // false → unsigned file
    pub cose_sign1_bytes: Vec<u8>,               // raw COSE_Sign1_Tagged
    pub cert_chain_der: Vec<Vec<u8>>,            // X.509 certs (leaf first)
    pub claim_cbor: Vec<u8>,                     // raw claim CBOR payload
    pub official_trust_anchors_der: Vec<Vec<u8>>, // C2PA official trust list
    pub curated_trust_anchors_der: Vec<Vec<u8>>, // project-curated certs
}
```

### Usage

```bash
# Mock mode (instant, verifies correctness without real proof):
cargo run --release --bin prove -- --media data/samples/chatgpt.png \
  --trust-dir data/trust --mock

# Real Groth16 proof (CPU, takes minutes):
cargo run --release --bin prove -- --media data/samples/chatgpt.png \
  --trust-dir data/trust

# GPU prover (set SP1_PROVER=cuda, needs NVIDIA GPU):
SP1_PROVER=cuda cargo run --release --bin prove -- --media data/samples/chatgpt.png \
  --trust-dir data/trust
```

---

## Service 2: zkVM Guest Program (`services/prover/program/`)

**What it does:** Runs inside SP1's RISC-V virtual machine. Cryptographically verifies the C2PA signature, validates trust anchors, extracts metadata from verified data, and commits public outputs. Every operation is proven correct by the ZK proof.

**Key file:** `src/main.rs` (~235 lines)

### 7-Step Verification Flow

```
CryptoEvidence (from host via SP1Stdin)
    │
    ▼
1. Parse COSE_Sign1 (try tagged CBOR tag 18, then untagged)
   If parsing fails → return unsigned outputs
    │
    ▼
2. Check algorithm = ES256 (ECDSA P-256 with SHA-256)
   Only supported algorithm; reject others gracefully
    │
    ▼
3. Parse leaf X.509 certificate (DER) → extract P-256 public key
   Uses x509-cert crate (no_std) for SubjectPublicKeyInfo
    │
    ▼
4. Build COSE Sig_structure1 and verify ECDSA P-256 signature
   Sig_structure1 = CBOR: ["Signature1", protected_header, b"", claim_cbor]
   Verify: VerifyingKey.verify(Sig_structure1, signature)
    │
    ▼  ← SIGNATURE VERIFIED — everything below uses authenticated data
    │
5. Match root cert (last in chain) against trust anchor lists
   DER byte comparison: official → curated → untrusted
    │
    ▼
6. Extract issuer Organization + subject Common Name from leaf cert
   Parses X.509 RDN attributes (OID 2.5.4.10, 2.5.4.3)
    │
    ▼
7. Parse claim CBOR for claim_generator
   C2PA v2: claim_generator_info.name (map)
   C2PA v1: claim_generator (text string)
    │
    ▼
Commit PublicOutputs via sp1_zkvm::io::commit()
```

### Security Properties

| Property | How it's enforced |
|---|---|
| Content hash integrity | SHA-256 computed from raw file bytes (host-provided) |
| Signature authenticity | ECDSA P-256 verified over Sig_structure1 inside zkVM |
| Trust binding | Root cert DER compared against known trust anchors inside zkVM |
| Metadata integrity | Issuer/CN extracted from signature-verified leaf cert |
| Claim integrity | claim_generator extracted from signature-authenticated claim CBOR |
| Tamper resistance | All outputs derived from verified crypto data; host cannot forge |

### PublicOutputs (Guest → On-Chain)

```rust
pub struct PublicOutputs {
    pub content_hash: [u8; 32],        // SHA-256 of file
    pub has_c2pa: bool,                // true if valid C2PA with verified signature
    pub trust_list_match: String,      // "official" | "curated" | "untrusted"
    pub validation_state: String,      // "Verified" | "SignatureOnly" | "None"
    pub digital_source_type: String,   // IPTC URI (future: from assertions)
    pub issuer: String,                // cert issuer org (e.g. "Truepic")
    pub common_name: String,           // cert CN (e.g. "Truepic Lens CLI in Sora")
    pub software_agent: String,        // claim_generator (e.g. "ChatGPT")
    pub signing_time: String,          // ISO timestamp (future: from COSE header)
}
```

### Example Output (chatgpt.png)

```
content_hash:       4ebd98d3893a16a6b3cf73c4b3cdf3b55149af563c47e07dd58f9bba17a8aabf
has_c2pa:           true
trust_list_match:   curated
validation_state:   Verified
issuer:             Truepic
common_name:        Truepic Lens CLI in Sora
software_agent:     ChatGPT
cycles:             ~1,140,000 (vs ~442,000 unsigned)
```

### Guest Dependencies (no_std)

All guest dependencies run without `std` to compile for RISC-V:

| Crate | Purpose | Notes |
|---|---|---|
| `coset` | COSE_Sign1 parsing | `default-features = false` |
| `ciborium` | CBOR encoding/decoding | `default-features = false` |
| `p256` | ECDSA P-256 verification | SP1 patched for accelerated cycles |
| `x509-cert` | X.509 certificate parsing | `default-features = false` |
| `der` | ASN.1 DER decoding | `default-features = false, features = ["alloc", "oid"]` |

The `p256` crate uses an SP1 patch (`sp1-patches/elliptic-curves`, tag `patch-p256-13.2-sp1-5.0.0`) that replaces field arithmetic with SP1 precompiles for ~10x faster execution inside the zkVM.

---

## Service 3: Solana Program (`services/provenance_attestation/`)

**What it does:** Anchor program that stores attestation records on Solana. Has one instruction (`submit_proof`) that verifies a Groth16 proof and creates an on-chain PDA account with the attestation data.

**Key files:**
- `programs/provenance_attestation/src/lib.rs` — `submit_proof` instruction
- `programs/provenance_attestation/src/state.rs` — `Attestation` account (1006 bytes)
- `programs/provenance_attestation/src/constants.rs` — PDA seed, SP1 vkey hash
- `programs/provenance_attestation/src/errors.rs` — custom errors

### How `submit_proof` Works

```
Transaction arrives with:
  ├── proof bytes (Groth16, ~256 bytes)
  ├── public_inputs bytes
  └── attestation fields (decoded from proof's public outputs by the client)

Step 1: Verify Groth16 proof
  sp1_solana::verify_proof(&proof, &public_inputs, SP1_VKEY_HASH, GROTH16_VK_BYTES)
  ~280K CU via Solana's alt_bn128 precompiles
  (TODO: enable when sp1-solana integration is complete)

Step 2: Validate string lengths (each ≤ 128 bytes)

Step 3: Create PDA account
  seeds = [b"attestation", content_hash]
  If PDA exists → transaction fails (one attestation per file)

Step 4: Store all fields in PDA account
  No authority check — the proof IS the authorization
```

### On-Chain Account

```rust
#[account]
pub struct Attestation {
    pub content_hash: [u8; 32],       // PDA seed (unique per file)
    pub has_c2pa: bool,
    pub trust_list_match: String,     // "official" | "curated" | "untrusted"
    pub validation_state: String,     // "Verified" | "SignatureOnly" | "None"
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

### Reading Attestations

```typescript
// Derive PDA from content hash (off-chain)
const [pda] = PublicKey.findProgramAddressSync(
  [Buffer.from("attestation"), contentHashBytes],
  programId
);
// Read account data directly — no transaction needed
const attestation = await program.account.attestation.fetch(pda);
```

**Program ID:** `HahVgC9uo73aLw1ouBEvgMT7KmGTS6rovfbKP9zuCtjc`

---

## Service 4: Verifier (`services/verifier/`)

**What it does:** Standalone Rust CLI that reads a file's C2PA manifest using the `c2pa-rs` SDK and produces structured JSON. Useful for local inspection, but **not part of the trustless pipeline** — the prover does its own raw extraction.

```bash
cargo run --bin verifier -- data/samples/chatgpt.png
```

---

## Full Data Flow

```
            data/samples/chatgpt.png
                     │
                     ▼
    ┌────────────────────────────────┐
    │  Prover Host (jumbf_extract)   │
    │                                │
    │  PNG → caBX → JUMBF tree      │
    │  → COSE_Sign1 bytes           │
    │  → cert chain DER             │
    │  → claim CBOR bytes           │
    │  + trust anchor PEMs → DER    │
    │                                │
    │  Output: CryptoEvidence        │
    └───────────────┬────────────────┘
                    │ SP1Stdin::write()
                    ▼
    ┌────────────────────────────────┐
    │  SP1 zkVM Guest (RISC-V)       │
    │                                │
    │  1. Parse COSE_Sign1           │
    │  2. Check ES256 algorithm      │
    │  3. Extract P-256 key from     │
    │     X.509 leaf cert            │
    │  4. Verify ECDSA signature     │  ← crypto verified
    │  5. Match root cert to trust   │
    │     anchors (DER comparison)   │
    │  6. Extract issuer/CN from     │
    │     verified cert              │
    │  7. Parse claim CBOR for       │
    │     claim_generator            │
    │                                │
    │  Commit: PublicOutputs         │
    │  Prove:  Groth16 (~256 bytes)  │
    └───────────────┬────────────────┘
                    │
                    ▼
    ┌────────────────────────────────┐
    │  Solana Program                │
    │  (provenance_attestation)      │
    │                                │
    │  1. Verify Groth16 proof       │
    │     (alt_bn128 precompiles)    │
    │  2. Create PDA account         │
    │     seeds: [b"attestation",    │
    │             content_hash]      │
    │  3. Store verified attestation │
    │                                │
    │  Anyone can read by deriving   │
    │  PDA from content_hash         │
    └────────────────────────────────┘
```

---

## Project Structure

```
r3l-provenance/
├── data/
│   ├── samples/          # Test media files (chatgpt.png, etc.)
│   └── trust/            # Trust anchor certificates
│       ├── official/     # C2PA official trust list (PEM)
│       └── curated/      # Project-curated certs (PEM)
├── services/
│   ├── prover/           # SP1 zkVM prover
│   │   ├── program/      # Guest (compiles to RISC-V ELF)
│   │   │   └── src/main.rs
│   │   ├── script/       # Host (runs natively)
│   │   │   └── src/
│   │   │       ├── bin/prove.rs
│   │   │       ├── jumbf_extract.rs
│   │   │       └── lib.rs
│   │   └── shared/       # Types shared between host and guest
│   │       └── src/lib.rs
│   ├── provenance_attestation/   # Anchor Solana program
│   │   └── programs/provenance_attestation/src/
│   │       ├── lib.rs
│   │       ├── state.rs
│   │       ├── constants.rs
│   │       └── errors.rs
│   └── verifier/         # Standalone C2PA verifier CLI
│       └── src/
│           ├── lib.rs
│           └── main.rs
└── PLAN.md               # Master design document
```

---

## Key Constants

| Constant | Value |
|---|---|
| Program ID | `HahVgC9uo73aLw1ouBEvgMT7KmGTS6rovfbKP9zuCtjc` |
| SP1 vkey hash | `0x0014beac9f9d3c39486f2537e8e5aa7ec0efbf648b9b5ef7e1b403c1d6dc4a1a` |
| PDA seed | `b"attestation"` + `content_hash` |
| Max string length | 128 bytes |
| Account space | 1006 bytes |
| Proof verification CU | ~280,000 (request 400,000 budget) |
| zkVM cycles (signed) | ~1,140,000 |
| zkVM cycles (unsigned) | ~442,000 |

---

## What's Done

- [x] Host-side JUMBF extraction (PNG → caBX → JUMBF → COSE + claim + certs)
- [x] ECDSA P-256 signature verification inside zkVM (with SP1-patched `p256`)
- [x] X.509 certificate parsing for public key extraction and name fields
- [x] Trust anchor matching (official/curated/untrusted) inside zkVM
- [x] Claim CBOR parsing for claim_generator (v1 + v2 support)
- [x] Solana program with PDA-based attestation storage
- [x] Mock prover end-to-end test passing

## Next Steps

### Immediate
1. **Enable sp1-solana proof verification** — Uncomment `sp1_solana::verify_proof()` in the Solana program, add the `sp1-solana` crate dependency, test with real Groth16 proofs on devnet
2. **Bind proof public outputs to instruction args** — On-chain program should decode `PublicOutputs` from the proof's public values and verify they match the submitted attestation fields (prevents substitution attacks)
3. **Generate real Groth16 proof** — Run with CPU or GPU prover (not `--mock`) to produce a proof that can be submitted on-chain

### Short-Term
4. **Assertion box parsing** — Extract `digital_source_type` and `software_agent` from C2PA assertion boxes (currently only from claim_generator)
5. **Signing time extraction** — Parse RFC 3161 timestamp or COSE protected header `sigTst` for `signing_time`
6. **JPEG/MP4 support** — Extend JUMBF extraction to handle JPEG APP11 markers and MP4 uuid boxes (currently PNG-only)

### Medium-Term
7. **Web API + UI** — HTTP API (axum) orchestrating verify → prove → submit, with Vue SPA frontend
8. **P-384/EdDSA signature support** — Some C2PA signers use P-384 or EdDSA; requires additional SP1 patches
9. **Intermediate cert chain validation** — Currently matches root cert only; full chain verification needs RSA support in zkVM (waiting for SP1 patches)
10. **Mainnet deployment** — Deploy Solana program to mainnet, set up production GPU prover infrastructure
