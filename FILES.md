# File-by-File Reference

Every source file in the project, what it does, and how other files depend on it.

---

## Shared Types — `services/prover/shared/`

### `shared/Cargo.toml`
Package definition for `prover-shared` crate. Depends only on `serde`.

**Used by:** `program/Cargo.toml` and `script/Cargo.toml` (both depend on `prover-shared = { path = "../shared" }`)

### `shared/src/lib.rs`
Defines `CryptoEvidence` and `PublicOutputs` — the two structs that form the data contract between the prover host and the zkVM guest.

- `CryptoEvidence`: Private inputs (asset hash, COSE signature, cert chain, claim CBOR, trust anchors). Serialized into `SP1Stdin` by the host, deserialized by the guest.
- `PublicOutputs`: Committed outputs (content hash, has_c2pa, trust level, issuer, etc.). Written by the guest via `sp1_zkvm::io::commit()`, read back by the host, and eventually stored on-chain.

**Used by:**
- `script/src/jumbf_extract.rs` — constructs `CryptoEvidence` from extracted PNG data
- `script/src/bin/prove.rs` — reads `PublicOutputs` from execution result
- `program/src/main.rs` — reads `CryptoEvidence` from stdin, commits `PublicOutputs`

---

## zkVM Guest Program — `services/prover/program/`

### `program/Cargo.toml`
Package definition for `provenance-program`. Depends on `sp1-zkvm`, `prover-shared`, and no_std crypto crates (`coset`, `ciborium`, `p256`, `x509-cert`, `der`). Contains `[patch.crates-io]` entries for SP1-patched `sha2` and `p256` (accelerated precompiles).

**Used by:** `script/build.rs` builds this crate into a RISC-V ELF. The ELF name `provenance-program` is referenced by `include_elf!()` in `prove.rs` and `vkey.rs`.

### `program/src/main.rs`
The zkVM guest entry point. All cryptographic verification happens here — nothing in this file trusts the host.

1. Reads `CryptoEvidence` from `sp1_zkvm::io::read()`
2. If unsigned → commits empty `PublicOutputs`
3. If signed → `verify_and_extract()`:
   - Parses `COSE_Sign1` (tagged/untagged) via `coset`
   - Checks algorithm is ES256
   - Parses leaf X.509 cert via `x509-cert`, extracts P-256 public key
   - Builds `Sig_structure1` CBOR, verifies ECDSA signature via `p256`
   - Matches root cert DER against trust anchors → trust level
   - Extracts issuer org + CN from X.509 RDN attributes via `der`
   - Parses claim CBOR for `claim_generator_info` (v2) / `claim_generator` (v1) via `ciborium`
4. Commits `PublicOutputs` via `sp1_zkvm::io::commit()`

**Used by:** Compiled to ELF by `script/build.rs`. Loaded at runtime by `prove.rs` and `vkey.rs` via `include_elf!("provenance-program")`.

---

## Prover Host — `services/prover/script/`

### `script/Cargo.toml`
Package definition for `prover-script`. Declares three binaries (`prove`, `vkey`, `debug_jumbf`). Depends on `sp1-sdk`, `sp1-prover`, `prover-shared`, and extraction crates (`coset`, `ciborium`, `pem`, `sha2`).

**Used by:** `script/build.rs` is invoked by Cargo during compilation.

### `script/build.rs`
Build script that compiles the guest program (`../program`) into a RISC-V ELF using `sp1_build::build_program()`. The resulting ELF is embedded into the host binary via `include_elf!()`.

**Used by:** Cargo runs this automatically before compiling any binary in the `script` crate.

### `script/src/lib.rs`
Crate root. Exposes `pub mod jumbf_extract`.

**Used by:** `prove.rs` imports `prover_script::jumbf_extract`.

### `script/src/jumbf_extract.rs`
Host-side extraction of raw cryptographic evidence from PNG files. Does no verification — only byte extraction.

- `extract_crypto_evidence(media_path, trust_dir)` → `CryptoEvidence`: top-level function that orchestrates everything
- `extract_c2pa_from_png(data)` → concatenated caBX chunk bytes
- `parse_boxes(data)` → `Vec<BmffBox>`: ISO BMFF box parser
- `parse_jumd_label(data)` → label string from JUMBF Description box
- `extract_claim_and_signature(jumbf)` → `(claim_cbor, cose_sign1)`: walks JUMBF tree to find active manifest's claim and signature boxes
- `extract_cert_chain_from_cose(cose_bytes)` → `Vec<Vec<u8>>`: parses COSE_Sign1 x5chain header (label 33) for DER cert bytes
- `load_trust_anchors_der(dir)` → `Vec<Vec<u8>>`: reads PEM files from a directory, converts to DER

**Used by:**
- `script/src/bin/prove.rs` — calls `extract_crypto_evidence()` to build the guest's input
- `script/src/lib.rs` — re-exports as public module

### `script/src/bin/prove.rs`
CLI binary that runs the full prover pipeline: extract → execute guest → generate Groth16 proof.

1. Parses CLI args (`--media`, `--trust-dir`, `--output`, `--mock`)
2. Calls `jumbf_extract::extract_crypto_evidence()` to build `CryptoEvidence`
3. Writes `CryptoEvidence` to `SP1Stdin`
4. Creates `ProverClient` (mock/cpu/cuda based on args and env)
5. Calls `client.setup(elf)` → `(pk, vk)`
6. Calls `client.execute(elf, &stdin)` → reads back `PublicOutputs`
7. Calls `client.prove(&pk, &stdin, SP1ProofMode::Groth16)` → proof
8. Verifies proof locally, saves to disk

**Used by:**
- Run directly via `cargo run --bin prove`
- Called by `services/api/src/routes/prove.rs` via `tokio::process::Command` shell-out

### `script/src/bin/vkey.rs`
Prints the SP1 verification key hash for the guest program. Output is used in `constants.rs` for on-chain proof verification.

**Used by:** Run manually; its output is copy-pasted into `services/provenance_attestation/.../constants.rs`.

### `script/src/bin/debug_jumbf.rs`
Debug tool that dumps the full JUMBF box tree from a PNG file. Includes its own copy of PNG/JUMBF parsing (not shared with `jumbf_extract.rs`) plus CBOR key dumping via `ciborium`.

**Used by:** Run manually for debugging. Standalone — no other file imports from it.

---

## Workspace Root — `services/prover/`

### `Cargo.toml`
Workspace definition. Members: `script`, `shared`. Excludes `program` (it's built separately by `sp1_build`). Contains `[patch.crates-io]` for `alloy-consensus`.

### `Cargo.lock`
Locked dependency versions for the workspace (`script` + `shared`).

### `program/Cargo.lock`
Separate lockfile for the guest program (not part of the workspace).

---

## Verifier — `services/verifier/`

### `Cargo.toml`
Package definition for `verifier` crate. Depends on `c2pa` (with `file_io` feature), `sha2`, `serde`, `serde_json`, `hex`, `anyhow`.

**Used by:** `services/api/Cargo.toml` depends on `verifier = { path = "../verifier" }`.

### `src/lib.rs`
C2PA verification library using the `c2pa-rs` SDK. Exports:

- `VerifyOutput` struct: all attestation fields (content_hash, has_c2pa, trust_list_match, issuer, etc.)
- `verify(path, trust_dir)` → `VerifyOutput`: reads file, computes SHA-256, opens C2PA manifest with trust anchors, extracts props
- `verify_with_env(path)` → `VerifyOutput`: convenience wrapper using `TRUST_DIR` env var
- Internal: `load_pems()`, `try_read()`, `is_trusted()`, `resolve_trust()`, `extract_props()`, `extract_cn()`

**Used by:**
- `src/main.rs` — calls `verify_with_env()` for CLI usage
- `services/api/src/routes/verify.rs` — calls `verifier::verify()` as a path dependency
- `services/api/src/routes/prove.rs` — calls `verifier::verify()` before shelling out to prover

### `src/main.rs`
Thin CLI wrapper. Takes a file path as arg, calls `verifier::verify_with_env()`, prints JSON.

**Used by:** Run directly via `cargo run --bin verifier -- <file>`. Not imported by other code.

---

## Web API — `services/api/`

### `Cargo.toml`
Package definition for `api`. Key dependencies: `axum`, `tokio`, `tower-http`, `solana-sdk v2`, `solana-rpc-client v2`, and `verifier = { path = "../verifier" }` (path dependency on the verifier crate).

### `Cargo.lock`
Locked dependency versions for the API.

### `src/main.rs`
Axum HTTP server entry point.

- Defines `AppState` (trust_dir, prover_dir, rpc_url, keypair_path, program_id — all from env vars)
- Mounts routes: `POST /api/verify`, `POST /api/prove`, `POST /api/submit`, `GET /api/attestation/{hash}`, `GET /api/health`
- Serves static files from `STATIC_DIR` (Vue build output) with SPA fallback
- 50 MB body limit, permissive CORS

**Used by:** Run directly. The Vue frontend proxies to this in dev via Vite config.

### `src/routes/mod.rs`
Re-exports route modules: `pub mod attestation`, `prove`, `submit`, `verify`.

**Used by:** `src/main.rs` references routes as `routes::verify::verify`, etc.

### `src/routes/verify.rs`
`POST /api/verify` handler. Accepts multipart file upload, writes to tempfile (preserving extension for c2pa format detection), calls `verifier::verify()` on a blocking thread, returns JSON.

**Uses:** `verifier::verify()` (path dependency), `crate::AppState`

**Used by:** `src/main.rs` mounts it. Called by `services/web/src/api.ts` → `verifyFile()`.

### `src/routes/prove.rs`
`POST /api/prove` handler. Accepts multipart file upload, first runs `verifier::verify()`, then shells out to `cargo run --bin prove -- --mock --json-out <sidecar>` in the prover directory. Returns proof hex + public outputs + verify result.

**Uses:** `verifier::verify()` (path dependency), `crate::AppState`, shells out to `prove` binary

**Used by:** `src/main.rs` mounts it. Called by `services/web/src/api.ts` → `proveFile()`.

### `src/routes/submit.rs`
`POST /api/submit` handler. Accepts JSON body with attestation fields + optional proof/public_inputs. Manually constructs an Anchor-compatible Solana transaction:
- Encodes Anchor `submit_proof` discriminator + borsh-serialized args
- Derives attestation PDA from content_hash
- Signs with keypair, sends via `RpcClient`

**Uses:** `crate::AppState`, `solana-sdk`, `solana-rpc-client`, `borsh`

**Used by:** `src/main.rs` mounts it. Called by `services/web/src/api.ts` → `submitAttestation()`.

### `src/routes/attestation.rs`
`GET /api/attestation/{hash}` handler. Derives PDA from hex content hash, fetches account via `RpcClient`, skips 8-byte Anchor discriminator, borsh-deserializes into `AttestationAccount` (local struct mirroring on-chain `Attestation`).

**Uses:** `crate::AppState`, `solana-sdk`, `solana-rpc-client`, `borsh`

**Used by:** `src/main.rs` mounts it. Called by `services/web/src/api.ts` → `lookupAttestation()`.

---

## Solana Program — `services/provenance_attestation/`

### `Anchor.toml`
Anchor workspace config. Program ID, cluster, wallet, test command.

### `Cargo.toml`
Workspace Cargo.toml for the Anchor project.

### `programs/provenance_attestation/Cargo.toml`
Package definition for the on-chain program. Depends on `anchor-lang 0.30.1` and `hex`.

### `programs/provenance_attestation/src/lib.rs`
Anchor program entry point. Declares program ID. Contains:

- `submit_proof` instruction: accepts proof bytes, public_inputs bytes, content_hash, and all attestation string fields. Validates string lengths, stores in PDA account. Groth16 verification is TODO (commented out `sp1_solana::verify_proof()`).
- `SubmitProof` accounts struct: `attestation` PDA (init, seeded by `[ATTESTATION_SEED, content_hash]`), `submitter` (signer/payer), `system_program`.

**Uses:** `constants.rs` (ATTESTATION_SEED, SP1_VKEY_HASH), `errors.rs` (ProvenanceError), `state.rs` (Attestation)

**Used by:**
- Deployed on-chain; called by `services/api/src/routes/submit.rs` and `scripts/submit.ts`
- Account data read by `services/api/src/routes/attestation.rs` and `scripts/submit.ts --query`

### `programs/provenance_attestation/src/state.rs`
Defines the `Attestation` account struct (Anchor `#[account]`). Fields: content_hash, has_c2pa, 7 string fields, submitted_by (Pubkey), timestamp, bump. Constants: `MAX_STRING_LEN = 128`, `SPACE = 1006`.

**Used by:** `lib.rs` (account type in `SubmitProof` accounts struct and field writes)

### `programs/provenance_attestation/src/constants.rs`
Defines `ATTESTATION_SEED` (`b"attestation"`) and `SP1_VKEY_HASH` (hex string from `vkey` binary output).

**Used by:** `lib.rs` (PDA derivation seed, proof verification key)

### `programs/provenance_attestation/src/errors.rs`
Defines `ProvenanceError` enum: `ProofVerificationFailed`, `InvalidPublicOutputs`, `StringTooLong`.

**Used by:** `lib.rs` (error returns from `submit_proof`)

### `scripts/submit.ts`
TypeScript CLI for submitting proofs and querying attestations. Uses `@coral-xyz/anchor` SDK. Reads IDL from `target/idl/`. Supports `--proof <file>` and `--query <hash>`.

**Uses:** Anchor IDL (from `anchor build`), `@coral-xyz/anchor`, `@solana/web3.js`

**Used by:** Run manually via `npx ts-node scripts/submit.ts`.

### `tests/provenance_attestation.ts`
Anchor integration tests. Three test cases:
1. Submit proof and verify stored attestation fields (using Anchor's camelCase field names)
2. Reject duplicate attestation for same content_hash
3. Reject strings exceeding MAX_STRING_LEN

**Uses:** `@coral-xyz/anchor`, `@solana/web3.js`, `chai`

**Used by:** Run via `anchor test`.

### `package.json` / `package-lock.json`
Node.js dependencies for tests and scripts (`@coral-xyz/anchor`, `@solana/web3.js`, `chai`, `mocha`, `ts-mocha`).

### `tsconfig.json`
TypeScript config for tests and scripts.

---

## Web Frontend — `services/web/`

### `package.json` / `package-lock.json`
Node.js dependencies: `vue`, `axios`, `vite`, `@vitejs/plugin-vue`, `@tailwindcss/vite`, `typescript`.

### `vite.config.ts`
Vite config. Plugins: Vue, Tailwind CSS. Dev proxy: `/api/*` → `http://localhost:3001` (the API server).

**Used by:** Vite dev server and build process.

### `tsconfig.json` / `tsconfig.app.json` / `tsconfig.node.json`
TypeScript config (split for app and node contexts per Vue convention).

### `index.html`
SPA entry point. Loads `src/main.ts`. Contains `<div id="app">` mount point.

**Used by:** Vite serves this in dev; built version is served by the API's static file handler.

### `src/main.ts`
Vue app bootstrap. Creates app from `App.vue`, imports `style.css`, mounts to `#app`.

**Uses:** `App.vue`, `style.css`

### `src/style.css`
Global CSS. Imports Tailwind via `@import "tailwindcss"`.

**Used by:** `main.ts` imports it.

### `src/types.ts`
TypeScript interfaces mirroring the Rust structs:
- `VerifyOutput` — mirrors `verifier::VerifyOutput`
- `ProveResponse` — mirrors `routes::prove::ProveResponse`
- `SubmitResponse` — mirrors `routes::submit::SubmitResponse`
- `AttestationResponse` — mirrors `routes::attestation::AttestationResponse`

**Used by:** `api.ts`, `App.vue`, `FileUpload.vue`, `Results.vue`, `SubmitPanel.vue`, `LookupForm.vue`

### `src/api.ts`
Axios HTTP client for the API. Functions:
- `verifyFile(file)` → `POST /api/verify` (multipart)
- `proveFile(file)` → `POST /api/prove` (multipart)
- `submitAttestation(params)` → `POST /api/submit` (JSON)
- `lookupAttestation(hash)` → `GET /api/attestation/{hash}`

**Uses:** `types.ts` (all interfaces)

**Used by:** `FileUpload.vue`, `ProofStatus.vue`, `SubmitPanel.vue`, `LookupForm.vue`

### `src/App.vue`
Root component. Manages top-level state (`verifyResult`, `uploadedFile`). Shows `FileUpload` or `Results` + `LookupForm`.

**Uses:** `types.ts` (VerifyOutput), `FileUpload.vue`, `Results.vue`, `LookupForm.vue`

### `src/components/FileUpload.vue`
Drag-and-drop file upload zone. Calls `verifyFile()` from `api.ts`, emits `verified` event with result + file.

**Uses:** `api.ts` (verifyFile), `types.ts` (VerifyOutput)

**Used by:** `App.vue`

### `src/components/Results.vue`
Displays verification results with trust badge (green/yellow/red/gray). Shows attestation fields. Contains `ProofStatus` and `SubmitPanel` as child components.

**Uses:** `types.ts` (VerifyOutput), `ProofStatus.vue`, `SubmitPanel.vue`

**Used by:** `App.vue`

### `src/components/ProofStatus.vue`
"Generate Proof" button. Calls `proveFile()` from `api.ts`, emits `proved` event with proof hex + public inputs.

**Uses:** `api.ts` (proveFile)

**Used by:** `Results.vue`

### `src/components/SubmitPanel.vue`
"Submit to Solana" button. Calls `submitAttestation()` from `api.ts` with verify result + proof data. Shows tx signature and PDA address on success.

**Uses:** `api.ts` (submitAttestation), `types.ts` (VerifyOutput)

**Used by:** `Results.vue`

### `src/components/LookupForm.vue`
Content hash search form. Calls `lookupAttestation()` from `api.ts`, displays stored attestation fields or "not found".

**Uses:** `api.ts` (lookupAttestation), `types.ts` (AttestationResponse)

**Used by:** `App.vue`

---

## Data Files — `data/`

### `data/samples/`
Test media files (PNG, PDF, MP4). Used as input to the verifier and prover CLIs.

**Used by:** `prove.rs --media data/samples/chatgpt.png`, `debug_jumbf.rs` (default path)

### `data/trust/official/*.pem`
C2PA official trust list certificates (9 PEM files: adobe, digicert, google, irdeto, ssl-com, tauth, trufo, vivo, xiaomi).

**Used by:**
- `jumbf_extract.rs` → `load_trust_anchors_der()` reads these, converts PEM→DER, packs into `CryptoEvidence.official_trust_anchors_der`
- `verifier/src/lib.rs` → `load_pems()` reads these for `c2pa-rs` trust anchor config

### `data/trust/curated/*.pem`
Project-curated certificates (3 PEM files: adobe-firefly, c2pa-test, openai-truepic).

**Used by:** Same as official — loaded by both `jumbf_extract.rs` and `verifier/src/lib.rs`.

### `data/trust/C2PA-TRUST-LIST.pem` / `data/trust/trust-list.pem`
Bundled trust list files. Not directly loaded by code (the code reads from `official/` and `curated/` subdirectories).

---

## Infrastructure — `infra/`

### `infra/gpu-prover/main.tf`
Terraform config for provisioning an EC2 GPU instance (for running the SP1 CUDA prover).

### `infra/gpu-prover/README.md`
Instructions for deploying and using the GPU prover on AWS.

---

## Root Config Files

### `.gitignore`
Ignores `target/`, `node_modules/`, `*.bin`, `.env`, `proof.bin`, `verify.json`, `data/fixtures/`.

### `.dockerignore`
Docker build ignore patterns.

### `docker-compose.yml`
Docker Compose config for running the API + verifier.

### `docker/api.Dockerfile` / `docker/dev.Dockerfile` / `docker/verifier.Dockerfile`
Dockerfiles for building service images.

### `scripts/add-trust.sh`
Shell script for adding trust anchor PEM files to `data/trust/`.

### `PLAN.md`
Master design document with project goals and phases.

### `ARCHITECTURE.md`
High-level architecture doc covering the full pipeline (prover → zkVM → Solana).

### `README.md`
Project overview.

### `RUNNING.md`
Instructions for building and running each service.

---

## Cross-Service Dependency Graph

```
data/trust/*.pem ──────────┬──────────────────────────────────┐
                           │                                  │
data/samples/*.png ────────┤                                  │
                           │                                  │
                           ▼                                  ▼
                   jumbf_extract.rs              verifier/src/lib.rs
                   (PEM→DER, PNG                 (c2pa-rs SDK)
                    parsing)                           │
                           │                           │
                           ▼                           │
                  shared/src/lib.rs                    │
                  (CryptoEvidence,                     │
                   PublicOutputs)                      │
                    │           │                      │
                    │           │                      │
                    ▼           ▼                      │
             prove.rs     program/main.rs              │
             (host)       (zkVM guest)                 │
                    │                                  │
                    │                                  │
                    ▼                                  ▼
              api/src/routes/prove.rs ──── api/src/routes/verify.rs
                    │                           │
                    │                           │
                    ▼                           ▼
              api/src/routes/submit.rs    api/src/main.rs
                    │                           │
                    ▼                           ▼
         provenance_attestation          web/src/api.ts
         (Solana program)                      │
              │       │                        ▼
              │       ▼               web/src/components/*.vue
              │  api/src/routes/
              │  attestation.rs
              │
              ▼
         scripts/submit.ts
         tests/*.ts
```
