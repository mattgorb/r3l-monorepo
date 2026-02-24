<script setup lang="ts">
import { ref } from 'vue'

const activeSection = ref('overview')

const sections = [
  { id: 'overview', label: 'Overview' },
  { id: 'architecture', label: 'Architecture' },
  { id: 'similarity', label: 'Similarity Search' },
  { id: 'endpoints', label: 'API Endpoints' },
  { id: 'onchain', label: 'On-Chain Program' },
  { id: 'database', label: 'Database' },
  { id: 'edge', label: 'Edge Nodes' },
  { id: 'infra', label: 'Infrastructure' },
  { id: 'roadmap', label: 'Roadmap' },
]
</script>

<template>
  <div class="flex gap-8">
    <!-- Sidebar nav -->
    <nav class="hidden md:block w-44 shrink-0 sticky top-6 self-start space-y-1">
      <button
        v-for="s in sections"
        :key="s.id"
        @click="activeSection = s.id"
        :class="[
          'block w-full text-left px-3 py-1.5 rounded text-sm transition-colors cursor-pointer',
          activeSection === s.id ? 'bg-gray-800 text-white font-medium' : 'text-gray-500 hover:text-gray-300'
        ]"
      >{{ s.label }}</button>
    </nav>

    <!-- Content -->
    <div class="flex-1 min-w-0 space-y-10">
      <div>
        <h1 class="text-2xl font-bold">R3L Developer Docs</h1>
        <p class="text-sm text-gray-500 mt-1">Internal reference — what's built and running today.</p>
      </div>

      <!-- Mobile section selector -->
      <select
        v-model="activeSection"
        class="md:hidden w-full bg-gray-900 border border-gray-700 rounded-lg px-3 py-2 text-sm"
      >
        <option v-for="s in sections" :key="s.id" :value="s.id">{{ s.label }}</option>
      </select>

      <!-- ============ OVERVIEW ============ -->
      <section v-show="activeSection === 'overview'" class="space-y-6">
        <h2 class="text-lg font-semibold border-b border-gray-800 pb-2">System Overview</h2>

        <div class="grid grid-cols-2 md:grid-cols-4 gap-3">
          <div class="bg-gray-900 rounded-lg border border-gray-800 p-4 text-center">
            <p class="text-2xl font-bold text-green-400">11</p>
            <p class="text-xs text-gray-500 mt-1">API Endpoints</p>
          </div>
          <div class="bg-gray-900 rounded-lg border border-gray-800 p-4 text-center">
            <p class="text-2xl font-bold text-purple-400">2</p>
            <p class="text-xs text-gray-500 mt-1">Solana Instructions</p>
          </div>
          <div class="bg-gray-900 rounded-lg border border-gray-800 p-4 text-center">
            <p class="text-2xl font-bold text-blue-400">3</p>
            <p class="text-xs text-gray-500 mt-1">Edge SDKs</p>
          </div>
          <div class="bg-gray-900 rounded-lg border border-gray-800 p-4 text-center">
            <p class="text-2xl font-bold text-yellow-400">5</p>
            <p class="text-xs text-gray-500 mt-1">Frontend Pages</p>
          </div>
        </div>

        <div class="bg-gray-900 rounded-lg border border-gray-800 p-5 space-y-3">
          <h3 class="text-sm font-semibold text-gray-200">What's Working End-to-End</h3>
          <ul class="space-y-2 text-sm text-gray-400">
            <li class="flex items-start gap-2"><span class="text-green-400 mt-0.5">&#10003;</span> Upload media (image, video, audio, PDF) and extract C2PA provenance metadata</li>
            <li class="flex items-start gap-2"><span class="text-green-400 mt-0.5">&#10003;</span> Validate signing certificates against official + curated trust lists</li>
            <li class="flex items-start gap-2"><span class="text-green-400 mt-0.5">&#10003;</span> Wallet identity binding with Ed25519 signature verification on-chain (Solana precompile)</li>
            <li class="flex items-start gap-2"><span class="text-green-400 mt-0.5">&#10003;</span> Single-transaction attestation: C2PA metadata + wallet sig + identity written to Solana PDA</li>
            <li class="flex items-start gap-2"><span class="text-green-400 mt-0.5">&#10003;</span> Structured trust verdict API (trusted / attested / unknown) for external integrations</li>
            <li class="flex items-start gap-2"><span class="text-green-400 mt-0.5">&#10003;</span> Edge node attestation via API key auth (Python, Rust, C SDKs)</li>
            <li class="flex items-start gap-2"><span class="text-green-400 mt-0.5">&#10003;</span> Postgres persistence with on-chain fallback reads</li>
            <li class="flex items-start gap-2"><span class="text-green-400 mt-0.5">&#10003;</span> Lookup any attestation by content hash (DB first, Solana fallback)</li>
            <li class="flex items-start gap-2"><span class="text-green-400 mt-0.5">&#10003;</span> Batch query endpoint (up to 50 hashes per request)</li>
            <li class="flex items-start gap-2"><span class="text-green-400 mt-0.5">&#10003;</span> Unified search: hash lookup + file upload with TLSH byte-level + MobileCLIP2-S0 cross-modal similarity</li>
            <li class="flex items-start gap-2"><span class="text-green-400 mt-0.5">&#10003;</span> pgvector cosine nearest-neighbor search for semantic content matching</li>
          </ul>
        </div>

        <div class="bg-gray-900 rounded-lg border border-yellow-800/40 p-5 space-y-3">
          <h3 class="text-sm font-semibold text-yellow-400">Built But Not Yet Deployed</h3>
          <ul class="space-y-2 text-sm text-gray-400">
            <li class="flex items-start gap-2"><span class="text-yellow-400 mt-0.5">&#9888;</span> ZK Groth16 proof generation (SP1 prover — needs GPU instance, g5.2xlarge)</li>
            <li class="flex items-start gap-2"><span class="text-yellow-400 mt-0.5">&#9888;</span> On-chain Groth16 verification via sp1-solana (instruction ready, no GPU prover in prod)</li>
            <li class="flex items-start gap-2"><span class="text-yellow-400 mt-0.5">&#9888;</span> Email domain verification (API + UI ready, SMTP not configured)</li>
          </ul>
        </div>
      </section>

      <!-- ============ ARCHITECTURE ============ -->
      <section v-show="activeSection === 'architecture'" class="space-y-6">
        <h2 class="text-lg font-semibold border-b border-gray-800 pb-2">Architecture</h2>

        <pre class="bg-gray-900 rounded-lg border border-gray-800 p-5 text-xs text-gray-400 overflow-x-auto leading-relaxed">
Browser / Edge SDK
       |
       v
  Python API (FastAPI + uvicorn, port 3001)
       |
       +---> Rust verifier binary (C2PA extraction + trust list validation)
       |
       +---> Solana RPC (localhost:8899 dev / devnet / mainnet-beta)
       |         |
       |         +---> Anchor program: submit_attestation / submit_proof
       |         |       - PDA: seeds = [b"attestation", sha256(file)]
       |         |       - Ed25519 precompile for wallet sig verification
       |         |       - sp1-solana Groth16 verifier (trustless path)
       |         |
       |         +---> On-chain read: deserialize PDA by content hash
       |
       +---> Postgres + pgvector (attestation cache + similarity embeddings)
       |
       +---> SP1 prover (mock mode locally / GPU mode on g5 EC2)
       |
       +---> Similarity engine (TLSH hashing + MobileCLIP2-S0 embeddings)
       |
  Vue 3 SPA (served as static files from /dist)
        </pre>

        <div class="bg-gray-900 rounded-lg border border-gray-800 p-5 space-y-4">
          <h3 class="text-sm font-semibold text-gray-200">Service Directory</h3>
          <div class="space-y-2 text-sm">
            <div class="flex gap-3 text-gray-400">
              <span class="text-blue-400 font-mono w-52 shrink-0">services/api-py/</span>
              <span>Python FastAPI server — all routes, Solana tx builder, DB layer</span>
            </div>
            <div class="flex gap-3 text-gray-400">
              <span class="text-blue-400 font-mono w-52 shrink-0">services/verifier/</span>
              <span>Rust binary — C2PA extraction via c2pa-rs, trust list validation, SHA-256 hashing</span>
            </div>
            <div class="flex gap-3 text-gray-400">
              <span class="text-blue-400 font-mono w-52 shrink-0">services/provenance_attestation/</span>
              <span>Anchor Solana program — on-chain attestation storage + verification</span>
            </div>
            <div class="flex gap-3 text-gray-400">
              <span class="text-blue-400 font-mono w-52 shrink-0">services/prover/</span>
              <span>SP1 zkVM prover — guest program (runs in VM) + host script (generates proof)</span>
            </div>
            <div class="flex gap-3 text-gray-400">
              <span class="text-blue-400 font-mono w-52 shrink-0">services/web/</span>
              <span>Vue 3 + TypeScript + Tailwind SPA</span>
            </div>
            <div class="flex gap-3 text-gray-400">
              <span class="text-blue-400 font-mono w-52 shrink-0">services/edge-nodes/</span>
              <span>Edge SDKs: Python (pip), Rust (cargo), C (make)</span>
            </div>
          </div>
        </div>

        <div class="bg-gray-900 rounded-lg border border-gray-800 p-5 space-y-3">
          <h3 class="text-sm font-semibold text-gray-200">Two Attestation Paths</h3>
          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div class="border border-green-800/40 rounded-lg p-4 space-y-2">
              <p class="text-sm font-semibold text-green-400">Trusted Verifier (live)</p>
              <p class="text-xs text-gray-500">R3L server runs the Rust verifier, extracts C2PA, and submits to Solana with its authority keypair. Fast (&lt;2s). Requires trusting R3L didn't fabricate results.</p>
              <p class="text-xs text-gray-600 font-mono">proof_type = "trusted_verifier"</p>
            </div>
            <div class="border border-purple-800/40 rounded-lg p-4 space-y-2">
              <p class="text-sm font-semibold text-purple-400">ZK Groth16 (built, needs GPU)</p>
              <p class="text-xs text-gray-500">C2PA verification runs inside SP1 zkVM. Produces a Groth16 proof verified on-chain by the Solana program. Fully trustless — no need to trust R3L. Requires ~10min on A10G GPU.</p>
              <p class="text-xs text-gray-600 font-mono">proof_type = "zk_groth16"</p>
            </div>
          </div>
        </div>
      </section>

      <!-- ============ SIMILARITY SEARCH ============ -->
      <section v-show="activeSection === 'similarity'" class="space-y-6">
        <h2 class="text-lg font-semibold border-b border-gray-800 pb-2">Similarity Search</h2>

        <p class="text-sm text-gray-400">Every attested file is fingerprinted using two complementary techniques. These fingerprints are stored alongside the attestation and enable content-based search: find matching or visually similar files across the entire attestation database.</p>

        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div class="bg-gray-900 rounded-lg border border-orange-800/40 p-5 space-y-3">
            <p class="text-sm font-semibold text-orange-400">TLSH (Byte-Level)</p>
            <p class="text-xs text-gray-500">Trend Micro Locality Sensitive Hash. Produces a ~70-char hex digest from raw file bytes. Near-identical files (minor edits, re-encodes) produce hashes with low distance scores. Works on any file type but only detects byte-level similarity.</p>
            <div class="text-xs text-gray-600 space-y-1 mt-2">
              <p><span class="text-gray-400">Distance 0</span> — identical bytes</p>
              <p><span class="text-gray-400">Distance &lt;100</span> — near-duplicate</p>
              <p><span class="text-gray-400">Distance &gt;300</span> — unrelated</p>
            </div>
          </div>
          <div class="bg-gray-900 rounded-lg border border-blue-800/40 p-5 space-y-3">
            <p class="text-sm font-semibold text-blue-400">CLIP Embedding (Semantic)</p>
            <p class="text-xs text-gray-500">MobileCLIP2-S0 produces a 512-dimensional vector in a shared image-text embedding space. Enables cross-modal similarity: a photo of a dog and a PDF about dogs will have high cosine similarity, even though their bytes are completely different.</p>
            <div class="text-xs text-gray-600 space-y-1 mt-2">
              <p><span class="text-gray-400">Similarity &ge;0.95</span> — visually identical</p>
              <p><span class="text-gray-400">Similarity &ge;0.60</span> — semantically related</p>
              <p><span class="text-gray-400">Similarity &lt;0.60</span> — unrelated</p>
            </div>
          </div>
        </div>

        <div class="bg-gray-900 rounded-lg border border-gray-800 p-5 space-y-3">
          <h3 class="text-sm font-semibold text-gray-200">Cross-Modal Embedding Pipeline</h3>
          <p class="text-xs text-gray-400">MobileCLIP2-S0 has both an image encoder and a text encoder that produce vectors in the same 512-dim space. This enables semantic search across different file types.</p>
          <div class="space-y-3 mt-3">
            <div class="flex gap-3 items-start">
              <span class="text-green-400 text-xs font-mono w-24 shrink-0 mt-0.5">image/*</span>
              <p class="text-xs text-gray-500">Decoded via PIL, preprocessed, passed through <span class="font-mono text-gray-400">encode_image()</span>. Direct visual embedding.</p>
            </div>
            <div class="flex gap-3 items-start">
              <span class="text-purple-400 text-xs font-mono w-24 shrink-0 mt-0.5">video/*</span>
              <p class="text-xs text-gray-500">8 frames sampled at evenly-spaced timestamps via ffmpeg. Each frame encoded with <span class="font-mono text-gray-400">encode_image()</span>. Frame embeddings are averaged and L2-normalized into a single 512-dim vector.</p>
            </div>
            <div class="flex gap-3 items-start">
              <span class="text-red-400 text-xs font-mono w-24 shrink-0 mt-0.5">application/pdf</span>
              <p class="text-xs text-gray-500">Text extracted via PyMuPDF (all pages concatenated, truncated to 1000 chars). Tokenized and encoded with <span class="font-mono text-gray-400">encode_text()</span>.</p>
            </div>
            <div class="flex gap-3 items-start">
              <span class="text-yellow-400 text-xs font-mono w-24 shrink-0 mt-0.5">text/*</span>
              <p class="text-xs text-gray-500">Plain text, CSV, Markdown, HTML (tags stripped). Decoded as UTF-8, truncated to 1000 chars, encoded with <span class="font-mono text-gray-400">encode_text()</span>.</p>
            </div>
          </div>
          <p class="text-xs text-gray-600 mt-3">The waterfall tries image encoding first, then video frame sampling, then text extraction. Returns <span class="font-mono">None</span> if nothing works (e.g. audio-only files).</p>
        </div>

        <div class="bg-gray-900 rounded-lg border border-gray-800 p-5 space-y-3">
          <h3 class="text-sm font-semibold text-gray-200">Match Classification</h3>
          <p class="text-xs text-gray-400">Each result is classified based on the combination of TLSH distance and CLIP cosine similarity:</p>
          <div class="overflow-x-auto mt-3">
            <table class="w-full text-xs">
              <thead>
                <tr class="text-left text-gray-500 border-b border-gray-800">
                  <th class="pb-2 pr-4">Match Type</th>
                  <th class="pb-2 pr-4">TLSH Distance</th>
                  <th class="pb-2 pr-4">CLIP Similarity</th>
                  <th class="pb-2">Meaning</th>
                </tr>
              </thead>
              <tbody class="text-gray-400">
                <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-semibold text-green-400">exact</td><td class="pr-4">0</td><td class="pr-4">&ge;0.95</td><td>Identical file bytes</td></tr>
                <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-semibold text-blue-400">near_duplicate</td><td class="pr-4">&le;100</td><td class="pr-4">&ge;0.85</td><td>Minor edits, re-encodes, metadata changes</td></tr>
                <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-semibold text-yellow-400">visual_match</td><td class="pr-4">any</td><td class="pr-4">&ge;0.60</td><td>Semantically similar content (cross-modal)</td></tr>
                <tr><td class="py-2 pr-4 font-semibold text-gray-500">unrelated</td><td class="pr-4">any</td><td class="pr-4">&lt;0.60</td><td>No meaningful similarity detected</td></tr>
              </tbody>
            </table>
          </div>
        </div>

        <div class="bg-gray-900 rounded-lg border border-gray-800 p-5 space-y-3">
          <h3 class="text-sm font-semibold text-gray-200">Search Strategy</h3>
          <ol class="text-xs text-gray-400 list-decimal ml-4 space-y-2">
            <li><span class="text-gray-300">Exact hash match:</span> SHA-256 lookup against the attestation database.</li>
            <li><span class="text-gray-300">TLSH scan:</span> Compare the query's TLSH hash against all stored TLSH hashes. No distance cutoff — all comparisons are returned and ranked.</li>
            <li><span class="text-gray-300">CLIP nearest neighbors:</span> pgvector cosine distance search against all stored 512-dim embeddings. Returns top 20 ordered by similarity.</li>
          </ol>
          <p class="text-xs text-gray-500 mt-2">Results from all three stages are merged, deduplicated, classified, and sorted — exact matches first, then near-duplicates, then visual matches, then unrelated. Capped at 20 results.</p>
        </div>

        <div class="bg-gray-900 rounded-lg border border-gray-800 p-5 space-y-3">
          <h3 class="text-sm font-semibold text-gray-200">Supported File Types</h3>
          <p class="text-xs text-gray-400">Similarity search accepts the same file types as verification and attestation:</p>
          <div class="flex flex-wrap gap-2 mt-2">
            <span class="bg-gray-800 text-gray-400 text-xs px-2 py-1 rounded font-mono">image/*</span>
            <span class="bg-gray-800 text-gray-400 text-xs px-2 py-1 rounded font-mono">video/*</span>
            <span class="bg-gray-800 text-gray-400 text-xs px-2 py-1 rounded font-mono">audio/*</span>
            <span class="bg-gray-800 text-gray-400 text-xs px-2 py-1 rounded font-mono">application/pdf</span>
            <span class="bg-gray-800 text-gray-400 text-xs px-2 py-1 rounded font-mono">text/plain</span>
            <span class="bg-gray-800 text-gray-400 text-xs px-2 py-1 rounded font-mono">text/csv</span>
            <span class="bg-gray-800 text-gray-400 text-xs px-2 py-1 rounded font-mono">text/markdown</span>
            <span class="bg-gray-800 text-gray-400 text-xs px-2 py-1 rounded font-mono">text/html</span>
          </div>
          <p class="text-xs text-gray-500 mt-2">Audio files get TLSH only (no CLIP embedding). All other types get both TLSH + CLIP.</p>
        </div>
      </section>

      <!-- ============ API ENDPOINTS ============ -->
      <section v-show="activeSection === 'endpoints'" class="space-y-6">
        <h2 class="text-lg font-semibold border-b border-gray-800 pb-2">API Endpoints</h2>
        <p class="text-sm text-gray-500">Base URL: <span class="font-mono text-gray-400">/api</span></p>

        <div class="space-y-3">
          <!-- Health -->
          <div class="bg-gray-900 rounded-lg border border-gray-800 p-4">
            <div class="flex items-center gap-3">
              <span class="bg-green-900/60 text-green-400 text-xs font-mono px-2 py-0.5 rounded">GET</span>
              <span class="font-mono text-sm text-gray-200">/api/health</span>
            </div>
            <p class="text-xs text-gray-500 mt-2">Health check. Returns <span class="font-mono">"ok"</span>.</p>
          </div>

          <!-- Verify -->
          <div class="bg-gray-900 rounded-lg border border-gray-800 p-4">
            <div class="flex items-center gap-3">
              <span class="bg-blue-900/60 text-blue-400 text-xs font-mono px-2 py-0.5 rounded">POST</span>
              <span class="font-mono text-sm text-gray-200">/api/verify</span>
            </div>
            <p class="text-xs text-gray-500 mt-2">Upload a media file. Runs the Rust verifier to extract C2PA metadata, compute SHA-256 content hash, and validate the signing certificate against trust lists.</p>
            <p class="text-xs text-gray-600 mt-1">Body: <span class="font-mono">multipart/form-data</span> with <span class="font-mono">file</span> field. Max 50MB. Supports image/*, video/*, audio/*, application/pdf, text/*.</p>
            <p class="text-xs text-gray-600 mt-1">Returns: content_hash, has_c2pa, trust_list_match, validation_state, digital_source_type, issuer, common_name, software_agent, signing_time, actions, ingredients.</p>
          </div>

          <!-- Attest -->
          <div class="bg-gray-900 rounded-lg border border-blue-800/40 p-4">
            <div class="flex items-center gap-3">
              <span class="bg-blue-900/60 text-blue-400 text-xs font-mono px-2 py-0.5 rounded">POST</span>
              <span class="font-mono text-sm text-gray-200">/api/attest</span>
              <span class="text-xs text-yellow-500">primary endpoint</span>
            </div>
            <p class="text-xs text-gray-500 mt-2">Full pipeline: verify file + optional wallet signature + optional email token, then write a single Solana transaction + DB row. Idempotent — returns existing attestation if already on-chain.</p>
            <p class="text-xs text-gray-600 mt-1">Body: <span class="font-mono">multipart/form-data</span> — <span class="font-mono">file</span> (required), <span class="font-mono">wallet_pubkey</span>, <span class="font-mono">wallet_message</span>, <span class="font-mono">wallet_signature</span>, <span class="font-mono">email_token</span> (all optional).</p>
            <p class="text-xs text-gray-600 mt-1">Wallet message format: <span class="font-mono">"R3L: attest &lt;content_hash_hex&gt;"</span></p>
            <p class="text-xs text-gray-600 mt-1">When wallet is provided: builds Ed25519 precompile instruction for on-chain signature verification.</p>
          </div>

          <!-- Attestation lookup -->
          <div class="bg-gray-900 rounded-lg border border-gray-800 p-4">
            <div class="flex items-center gap-3">
              <span class="bg-green-900/60 text-green-400 text-xs font-mono px-2 py-0.5 rounded">GET</span>
              <span class="font-mono text-sm text-gray-200">/api/attestation/{content_hash}</span>
            </div>
            <p class="text-xs text-gray-500 mt-2">Lookup a single attestation by SHA-256 content hash. Checks Postgres first, falls back to on-chain PDA deserialization.</p>
          </div>

          <!-- List attestations -->
          <div class="bg-gray-900 rounded-lg border border-gray-800 p-4">
            <div class="flex items-center gap-3">
              <span class="bg-green-900/60 text-green-400 text-xs font-mono px-2 py-0.5 rounded">GET</span>
              <span class="font-mono text-sm text-gray-200">/api/attestations</span>
            </div>
            <p class="text-xs text-gray-500 mt-2">List all attestations, ordered by timestamp descending. Returns summary items.</p>
          </div>

          <!-- Query API -->
          <div class="bg-gray-900 rounded-lg border border-green-800/40 p-4">
            <div class="flex items-center gap-3">
              <span class="bg-green-900/60 text-green-400 text-xs font-mono px-2 py-0.5 rounded">GET</span>
              <span class="font-mono text-sm text-gray-200">/api/v1/query/{content_hash}</span>
              <span class="text-xs text-green-500">Truth Oracle</span>
            </div>
            <p class="text-xs text-gray-500 mt-2">Structured trust verdict for external consumers and AI agents. Returns a computed verdict: <span class="font-mono text-green-400">trusted</span> (C2PA official/curated signer), <span class="font-mono text-blue-400">attested</span> (on-chain but unknown signer), or <span class="font-mono text-gray-400">unknown</span> (no attestation).</p>
            <p class="text-xs text-gray-600 mt-1">Response includes: verdict, c2pa details, identity (wallet + email), signer info, proof type, attestation metadata.</p>
          </div>

          <!-- Batch query -->
          <div class="bg-gray-900 rounded-lg border border-green-800/40 p-4">
            <div class="flex items-center gap-3">
              <span class="bg-blue-900/60 text-blue-400 text-xs font-mono px-2 py-0.5 rounded">POST</span>
              <span class="font-mono text-sm text-gray-200">/api/v1/query/batch</span>
            </div>
            <p class="text-xs text-gray-500 mt-2">Batch query up to 50 content hashes. Returns array of verdicts. Body: JSON array of hex hash strings.</p>
          </div>

          <!-- Similar by file -->
          <div class="bg-gray-900 rounded-lg border border-orange-800/40 p-4">
            <div class="flex items-center gap-3">
              <span class="bg-blue-900/60 text-blue-400 text-xs font-mono px-2 py-0.5 rounded">POST</span>
              <span class="font-mono text-sm text-gray-200">/api/v1/similar</span>
              <span class="text-xs text-orange-400">Similarity Search</span>
            </div>
            <p class="text-xs text-gray-500 mt-2">Upload a file and find similar attested content. Computes TLSH hash + CLIP embedding from the upload, then searches against all stored attestations using byte-level and semantic similarity.</p>
            <p class="text-xs text-gray-600 mt-1">Body: <span class="font-mono">multipart/form-data</span> with <span class="font-mono">file</span> field. Supports image/*, video/*, audio/*, application/pdf, text/*.</p>
            <p class="text-xs text-gray-600 mt-1">Returns: query_hash, query_tlsh, and matches array (each with content_hash, match_type, tlsh_distance, clip_similarity, issuer, trust_list_match, has_c2pa, timestamp).</p>
          </div>

          <!-- Similar by hash -->
          <div class="bg-gray-900 rounded-lg border border-orange-800/40 p-4">
            <div class="flex items-center gap-3">
              <span class="bg-green-900/60 text-green-400 text-xs font-mono px-2 py-0.5 rounded">GET</span>
              <span class="font-mono text-sm text-gray-200">/api/v1/similar/{content_hash}</span>
              <span class="text-xs text-orange-400">Similarity Search</span>
            </div>
            <p class="text-xs text-gray-500 mt-2">Find content similar to an existing attestation by its SHA-256 content hash. Uses the stored TLSH hash and CLIP embedding from the original attestation to search against all other entries.</p>
            <p class="text-xs text-gray-600 mt-1">Returns: query_hash, query_tlsh, and matches array. Same response format as the POST endpoint.</p>
          </div>

          <!-- Edge register -->
          <div class="bg-gray-900 rounded-lg border border-purple-800/40 p-4">
            <div class="flex items-center gap-3">
              <span class="bg-blue-900/60 text-blue-400 text-xs font-mono px-2 py-0.5 rounded">POST</span>
              <span class="font-mono text-sm text-gray-200">/api/edge/register</span>
              <span class="text-xs text-purple-400">API key auth</span>
            </div>
            <p class="text-xs text-gray-500 mt-2">Register an edge node wallet. Verifies Ed25519 signature off-chain to prove wallet ownership. Returns API key for subsequent attestation calls.</p>
            <p class="text-xs text-gray-600 mt-1">Body: <span class="font-mono">{ pubkey, message, signature, name? }</span></p>
          </div>

          <!-- Edge attest -->
          <div class="bg-gray-900 rounded-lg border border-purple-800/40 p-4">
            <div class="flex items-center gap-3">
              <span class="bg-blue-900/60 text-blue-400 text-xs font-mono px-2 py-0.5 rounded">POST</span>
              <span class="font-mono text-sm text-gray-200">/api/edge/attest</span>
              <span class="text-xs text-purple-400">API key auth</span>
            </div>
            <p class="text-xs text-gray-500 mt-2">Submit attestation from an edge node. Requires <span class="font-mono">X-API-Key</span> header. Wallet resolved from customer record. Optional <span class="font-mono">wallet_signature</span> for on-chain Ed25519 verification.</p>
            <p class="text-xs text-gray-600 mt-1">Body: <span class="font-mono">{ content_hash, has_c2pa, trust_list_match, issuer, ... }</span></p>
          </div>

          <!-- Prove -->
          <div class="bg-gray-900 rounded-lg border border-gray-800 p-4 opacity-60">
            <div class="flex items-center gap-3">
              <span class="bg-blue-900/60 text-blue-400 text-xs font-mono px-2 py-0.5 rounded">POST</span>
              <span class="font-mono text-sm text-gray-200">/api/prove</span>
              <span class="text-xs text-yellow-500">needs GPU</span>
            </div>
            <p class="text-xs text-gray-500 mt-2">Generate SP1 Groth16 proof of C2PA verification. Currently runs in mock mode. Requires g5.2xlarge EC2 with CUDA for real proofs.</p>
          </div>

          <!-- Submit proof -->
          <div class="bg-gray-900 rounded-lg border border-gray-800 p-4 opacity-60">
            <div class="flex items-center gap-3">
              <span class="bg-blue-900/60 text-blue-400 text-xs font-mono px-2 py-0.5 rounded">POST</span>
              <span class="font-mono text-sm text-gray-200">/api/submit</span>
              <span class="text-xs text-yellow-500">needs GPU</span>
            </div>
            <p class="text-xs text-gray-500 mt-2">Submit a pre-generated Groth16 proof + public inputs to the Solana program for on-chain verification. Trustless path.</p>
          </div>
        </div>
      </section>

      <!-- ============ ON-CHAIN PROGRAM ============ -->
      <section v-show="activeSection === 'onchain'" class="space-y-6">
        <h2 class="text-lg font-semibold border-b border-gray-800 pb-2">On-Chain Program (Solana)</h2>

        <div class="bg-gray-900 rounded-lg border border-gray-800 p-5 space-y-3">
          <h3 class="text-sm font-semibold text-gray-200">Program Details</h3>
          <div class="space-y-2 text-sm">
            <div class="flex gap-3"><span class="text-gray-500 w-36 shrink-0">Framework</span><span class="text-gray-300">Anchor 0.30.1</span></div>
            <div class="flex gap-3"><span class="text-gray-500 w-36 shrink-0">Program ID</span><span class="text-gray-300 font-mono text-xs break-all">Derived from build keypair (varies per deployment)</span></div>
            <div class="flex gap-3"><span class="text-gray-500 w-36 shrink-0">PDA Seeds</span><span class="text-gray-300 font-mono text-xs">[b"attestation", content_hash_bytes]</span></div>
            <div class="flex gap-3"><span class="text-gray-500 w-36 shrink-0">Account Size</span><span class="text-gray-300 font-mono text-xs">~1,794 bytes per attestation</span></div>
          </div>
        </div>

        <div class="bg-gray-900 rounded-lg border border-gray-800 p-5 space-y-3">
          <h3 class="text-sm font-semibold text-gray-200">Instructions</h3>
          <div class="space-y-4">
            <div class="border-l-2 border-green-700 pl-4">
              <p class="text-sm font-semibold text-green-400 font-mono">submit_attestation</p>
              <p class="text-xs text-gray-500 mt-1">Authority-gated. R3L server submits verification results. If wallet is provided, scans transaction for Ed25519 precompile instruction and extracts the 64-byte signature for on-chain storage.</p>
              <p class="text-xs text-gray-600 mt-1">Accounts: attestation PDA (mut), authority (signer, mut), system_program, instructions_sysvar</p>
            </div>
            <div class="border-l-2 border-purple-700 pl-4">
              <p class="text-sm font-semibold text-purple-400 font-mono">submit_proof</p>
              <p class="text-xs text-gray-500 mt-1">Permissionless. Anyone can submit a Groth16 proof. The program verifies the proof on-chain using the SP1 verification key, then extracts and stores C2PA metadata from the public outputs.</p>
              <p class="text-xs text-gray-600 mt-1">Accounts: attestation PDA (mut), payer (signer, mut), system_program, instructions_sysvar</p>
            </div>
          </div>
        </div>

        <div class="bg-gray-900 rounded-lg border border-gray-800 p-5 space-y-3">
          <h3 class="text-sm font-semibold text-gray-200">Attestation PDA Fields</h3>
          <div class="overflow-x-auto">
            <table class="w-full text-xs">
              <thead>
                <tr class="text-left text-gray-500 border-b border-gray-800">
                  <th class="pb-2 pr-4">Field</th>
                  <th class="pb-2 pr-4">Type</th>
                  <th class="pb-2">Description</th>
                </tr>
              </thead>
              <tbody class="text-gray-400">
                <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono text-gray-300">content_hash</td><td class="pr-4">[u8; 32]</td><td>SHA-256 of original file bytes</td></tr>
                <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono text-gray-300">has_c2pa</td><td class="pr-4">bool</td><td>Whether file had valid C2PA metadata</td></tr>
                <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono text-gray-300">trust_list_match</td><td class="pr-4">String</td><td>"official", "curated", or "untrusted"</td></tr>
                <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono text-gray-300">validation_state</td><td class="pr-4">String</td><td>"Trusted", "Valid", or "Invalid"</td></tr>
                <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono text-gray-300">digital_source_type</td><td class="pr-4">String</td><td>IPTC digital source type URI</td></tr>
                <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono text-gray-300">issuer</td><td class="pr-4">String</td><td>Certificate issuer organization</td></tr>
                <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono text-gray-300">common_name</td><td class="pr-4">String</td><td>Certificate common name</td></tr>
                <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono text-gray-300">software_agent</td><td class="pr-4">String</td><td>Content creation tool</td></tr>
                <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono text-gray-300">signing_time</td><td class="pr-4">String</td><td>ISO timestamp of C2PA signature</td></tr>
                <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono text-gray-300">cert_fingerprint</td><td class="pr-4">String</td><td>SHA-256 of leaf signing certificate</td></tr>
                <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono text-gray-300">wallet</td><td class="pr-4">Pubkey</td><td>Creator's Solana wallet (zeros if none)</td></tr>
                <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono text-purple-400">wallet_sig</td><td class="pr-4">[u8; 64]</td><td>Ed25519 signature verified on-chain via precompile</td></tr>
                <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono text-gray-300">email_domain</td><td class="pr-4">String</td><td>Verified email domain (e.g. "nytimes.com")</td></tr>
                <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono text-gray-300">email_hash</td><td class="pr-4">[u8; 32]</td><td>SHA-256 of full email (privacy-preserving)</td></tr>
                <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono text-gray-300">proof_type</td><td class="pr-4">String</td><td>"trusted_verifier" or "zk_groth16"</td></tr>
                <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono text-gray-300">submitted_by</td><td class="pr-4">Pubkey</td><td>Transaction signer</td></tr>
                <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono text-gray-300">timestamp</td><td class="pr-4">i64</td><td>Solana clock timestamp</td></tr>
                <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono text-gray-300">verifier_version</td><td class="pr-4">String</td><td>Verifier binary version (e.g. "0.1.0")</td></tr>
                <tr><td class="py-2 pr-4 font-mono text-gray-300">trust_bundle_hash</td><td class="pr-4">String</td><td>SHA-256 of concatenated trust list PEMs</td></tr>
              </tbody>
            </table>
          </div>
        </div>

        <div class="bg-gray-900 rounded-lg border border-purple-800/40 p-5 space-y-3">
          <h3 class="text-sm font-semibold text-purple-400">Ed25519 On-Chain Verification</h3>
          <p class="text-xs text-gray-400">When a wallet signs an attestation, the transaction includes two instructions:</p>
          <ol class="text-xs text-gray-500 list-decimal ml-4 space-y-1 mt-2">
            <li><span class="text-gray-400">Ed25519 precompile instruction</span> — contains pubkey (32B), signature (64B), and message. Solana runtime rejects the tx if the signature is invalid.</li>
            <li><span class="text-gray-400">Attestation instruction</span> — our program reads the instructions sysvar, confirms the Ed25519 instruction is present, extracts the signature, and stores it in the PDA.</li>
          </ol>
          <p class="text-xs text-gray-500 mt-2">Message format: <span class="font-mono text-gray-400">"R3L: attest " + hex(content_hash)</span> (76 bytes total)</p>
          <p class="text-xs text-gray-500">Anyone can independently verify: read the PDA, reconstruct the message from content_hash, and verify the Ed25519 signature against the stored wallet pubkey. Zero trust in R3L required.</p>
        </div>
      </section>

      <!-- ============ DATABASE ============ -->
      <section v-show="activeSection === 'database'" class="space-y-6">
        <h2 class="text-lg font-semibold border-b border-gray-800 pb-2">Database (Postgres)</h2>

        <div class="bg-gray-900 rounded-lg border border-gray-800 p-5 space-y-3">
          <h3 class="text-sm font-semibold text-gray-200">Connection</h3>
          <div class="space-y-2 text-sm">
            <div class="flex gap-3"><span class="text-gray-500 w-36 shrink-0">Engine</span><span class="text-gray-300">PostgreSQL 16 (Alpine)</span></div>
            <div class="flex gap-3"><span class="text-gray-500 w-36 shrink-0">Driver</span><span class="text-gray-300">asyncpg (async) via SQLAlchemy 2.0</span></div>
            <div class="flex gap-3"><span class="text-gray-500 w-36 shrink-0">Extensions</span><span class="text-gray-300">pgvector (VECTOR type for CLIP embeddings + cosine distance search)</span></div>
            <div class="flex gap-3"><span class="text-gray-500 w-36 shrink-0">Pool Size</span><span class="text-gray-300">5 connections</span></div>
            <div class="flex gap-3"><span class="text-gray-500 w-36 shrink-0">Dev URL</span><span class="text-gray-300 font-mono text-xs">postgresql://postgres:postgres@localhost:5432/r3l</span></div>
            <div class="flex gap-3"><span class="text-gray-500 w-36 shrink-0">Prod</span><span class="text-gray-300">RDS db.t4g.micro (20GB gp3, single-AZ)</span></div>
          </div>
        </div>

        <div class="bg-gray-900 rounded-lg border border-gray-800 p-5 space-y-3">
          <h3 class="text-sm font-semibold text-gray-200">Tables</h3>

          <div class="mt-3">
            <p class="text-sm font-mono text-blue-400 mb-2">attestations</p>
            <p class="text-xs text-gray-500 mb-3">Mirrors on-chain PDA data for fast reads. Content hash is unique key. On-chain is the source of truth — DB is the read cache.</p>
            <div class="overflow-x-auto">
              <table class="w-full text-xs">
                <thead>
                  <tr class="text-left text-gray-500 border-b border-gray-800">
                    <th class="pb-2 pr-4">Column</th>
                    <th class="pb-2 pr-4">Type</th>
                    <th class="pb-2">Notes</th>
                  </tr>
                </thead>
                <tbody class="text-gray-400">
                  <tr class="border-b border-gray-800/50"><td class="py-1.5 pr-4 font-mono">id</td><td class="pr-4">INTEGER PK</td><td>Auto-increment</td></tr>
                  <tr class="border-b border-gray-800/50"><td class="py-1.5 pr-4 font-mono">content_hash</td><td class="pr-4">VARCHAR UNIQUE</td><td>SHA-256 hex, lookup key</td></tr>
                  <tr class="border-b border-gray-800/50"><td class="py-1.5 pr-4 font-mono">proof_type</td><td class="pr-4">VARCHAR</td><td>"trusted_verifier" or "zk_groth16"</td></tr>
                  <tr class="border-b border-gray-800/50"><td class="py-1.5 pr-4 font-mono">tx_signature</td><td class="pr-4">VARCHAR</td><td>Solana transaction signature</td></tr>
                  <tr class="border-b border-gray-800/50"><td class="py-1.5 pr-4 font-mono">pda</td><td class="pr-4">VARCHAR</td><td>Attestation PDA address</td></tr>
                  <tr class="border-b border-gray-800/50"><td class="py-1.5 pr-4 font-mono">has_c2pa</td><td class="pr-4">BOOLEAN</td><td></td></tr>
                  <tr class="border-b border-gray-800/50"><td class="py-1.5 pr-4 font-mono">trust_list_match</td><td class="pr-4">VARCHAR</td><td>official / curated / untrusted</td></tr>
                  <tr class="border-b border-gray-800/50"><td class="py-1.5 pr-4 font-mono">validation_state</td><td class="pr-4">VARCHAR</td><td></td></tr>
                  <tr class="border-b border-gray-800/50"><td class="py-1.5 pr-4 font-mono">digital_source_type</td><td class="pr-4">VARCHAR</td><td>IPTC URI</td></tr>
                  <tr class="border-b border-gray-800/50"><td class="py-1.5 pr-4 font-mono">issuer</td><td class="pr-4">VARCHAR</td><td></td></tr>
                  <tr class="border-b border-gray-800/50"><td class="py-1.5 pr-4 font-mono">common_name</td><td class="pr-4">VARCHAR</td><td></td></tr>
                  <tr class="border-b border-gray-800/50"><td class="py-1.5 pr-4 font-mono">software_agent</td><td class="pr-4">VARCHAR</td><td></td></tr>
                  <tr class="border-b border-gray-800/50"><td class="py-1.5 pr-4 font-mono">signing_time</td><td class="pr-4">VARCHAR</td><td></td></tr>
                  <tr class="border-b border-gray-800/50"><td class="py-1.5 pr-4 font-mono">cert_fingerprint</td><td class="pr-4">VARCHAR</td><td></td></tr>
                  <tr class="border-b border-gray-800/50"><td class="py-1.5 pr-4 font-mono">email_domain</td><td class="pr-4">VARCHAR</td><td>Nullable</td></tr>
                  <tr class="border-b border-gray-800/50"><td class="py-1.5 pr-4 font-mono">wallet_pubkey</td><td class="pr-4">VARCHAR</td><td>Base58 Solana pubkey, nullable</td></tr>
                  <tr class="border-b border-gray-800/50"><td class="py-1.5 pr-4 font-mono">submitted_by</td><td class="pr-4">VARCHAR</td><td>Edge node name or null</td></tr>
                  <tr class="border-b border-gray-800/50"><td class="py-1.5 pr-4 font-mono">verifier_version</td><td class="pr-4">VARCHAR</td><td></td></tr>
                  <tr class="border-b border-gray-800/50"><td class="py-1.5 pr-4 font-mono">trust_bundle_hash</td><td class="pr-4">VARCHAR</td><td></td></tr>
                  <tr class="border-b border-gray-800/50"><td class="py-1.5 pr-4 font-mono text-orange-400">tlsh_hash</td><td class="pr-4">VARCHAR</td><td>TLSH locality-sensitive hash (~70 hex chars). Nullable.</td></tr>
                  <tr class="border-b border-gray-800/50"><td class="py-1.5 pr-4 font-mono text-orange-400">clip_embedding</td><td class="pr-4">VECTOR(512)</td><td>MobileCLIP2-S0 L2-normalized embedding. pgvector type. Nullable.</td></tr>
                  <tr><td class="py-1.5 pr-4 font-mono">created_at</td><td class="pr-4">BIGINT</td><td>Unix timestamp</td></tr>
                </tbody>
              </table>
            </div>
          </div>

          <div class="mt-6">
            <p class="text-sm font-mono text-blue-400 mb-2">customers</p>
            <p class="text-xs text-gray-500 mb-3">Edge node registry. Each customer has a wallet + API key for authenticated attestations.</p>
            <div class="overflow-x-auto">
              <table class="w-full text-xs">
                <thead>
                  <tr class="text-left text-gray-500 border-b border-gray-800">
                    <th class="pb-2 pr-4">Column</th>
                    <th class="pb-2 pr-4">Type</th>
                    <th class="pb-2">Notes</th>
                  </tr>
                </thead>
                <tbody class="text-gray-400">
                  <tr class="border-b border-gray-800/50"><td class="py-1.5 pr-4 font-mono">id</td><td class="pr-4">INTEGER PK</td><td></td></tr>
                  <tr class="border-b border-gray-800/50"><td class="py-1.5 pr-4 font-mono">name</td><td class="pr-4">VARCHAR</td><td></td></tr>
                  <tr class="border-b border-gray-800/50"><td class="py-1.5 pr-4 font-mono">wallet_pubkey</td><td class="pr-4">VARCHAR UNIQUE</td><td>Base58</td></tr>
                  <tr class="border-b border-gray-800/50"><td class="py-1.5 pr-4 font-mono">api_key</td><td class="pr-4">VARCHAR UNIQUE</td><td>UUID v4</td></tr>
                  <tr><td class="py-1.5 pr-4 font-mono">created_at</td><td class="pr-4">BIGINT</td><td>Unix timestamp</td></tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>

        <div class="bg-gray-900 rounded-lg border border-gray-800 p-5 space-y-2">
          <h3 class="text-sm font-semibold text-gray-200">Read Strategy</h3>
          <p class="text-xs text-gray-400">All lookup endpoints follow the same pattern: Postgres first, Solana on-chain fallback. This gives fast reads (~1ms) in the common case while ensuring nothing is lost if the DB is reset. The on-chain PDA is the source of truth.</p>
          <p class="text-xs text-gray-500 mt-2">dev.sh resets the attestations table on each restart (schema changes don't auto-migrate). On-chain data persists across restarts via the test validator's --reset flag creating fresh state.</p>
        </div>
      </section>

      <!-- ============ EDGE NODES ============ -->
      <section v-show="activeSection === 'edge'" class="space-y-6">
        <h2 class="text-lg font-semibold border-b border-gray-800 pb-2">Edge Node SDKs</h2>

        <p class="text-sm text-gray-400">Edge nodes are external devices/services that attest content via the API. They register a wallet, get an API key, and submit attestations programmatically.</p>

        <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div class="bg-gray-900 rounded-lg border border-blue-800/40 p-5 space-y-3">
            <p class="text-sm font-semibold text-blue-400">Python SDK</p>
            <p class="text-xs text-gray-500">pip-installable package with CLI and programmatic API. Bundles the Rust verifier binary for C2PA-aware attestations.</p>
            <div class="text-xs text-gray-600 space-y-1 mt-2">
              <p class="font-mono">pip install ./services/edge-nodes/python</p>
              <p class="font-mono">r3l-edge register</p>
              <p class="font-mono">r3l-edge attest photo.jpg</p>
            </div>
          </div>
          <div class="bg-gray-900 rounded-lg border border-orange-800/40 p-5 space-y-3">
            <p class="text-sm font-semibold text-orange-400">Rust SDK</p>
            <p class="text-xs text-gray-500">Compiled binary with clap CLI. Uses ed25519-dalek for signing, reqwest for HTTP. Production-grade performance.</p>
            <div class="text-xs text-gray-600 space-y-1 mt-2">
              <p class="font-mono">cd services/edge-nodes/rust</p>
              <p class="font-mono">cargo build --release</p>
              <p class="font-mono">./r3l-edge attest photo.jpg</p>
            </div>
          </div>
          <div class="bg-gray-900 rounded-lg border border-gray-700 p-5 space-y-3">
            <p class="text-sm font-semibold text-gray-300">C SDK</p>
            <p class="text-xs text-gray-500">Minimal C client for IoT/embedded. Uses OpenSSL for SHA-256 + Ed25519, libcurl for HTTP. No C2PA extraction (hash + attest only).</p>
            <div class="text-xs text-gray-600 space-y-1 mt-2">
              <p class="font-mono">cd services/edge-nodes/c</p>
              <p class="font-mono">make</p>
              <p class="font-mono">./r3l_edge attest photo.jpg</p>
            </div>
          </div>
        </div>

        <div class="bg-gray-900 rounded-lg border border-gray-800 p-5 space-y-3">
          <h3 class="text-sm font-semibold text-gray-200">Edge Protocol</h3>
          <ol class="text-xs text-gray-400 list-decimal ml-4 space-y-2">
            <li><span class="text-gray-300">Register:</span> Sign a message with Ed25519 keypair, POST to <span class="font-mono">/api/edge/register</span>. Get back an API key.</li>
            <li><span class="text-gray-300">Attest:</span> SHA-256 hash the file, sign <span class="font-mono">"R3L: attest &lt;hash&gt;"</span>, POST to <span class="font-mono">/api/edge/attest</span> with X-API-Key header.</li>
            <li><span class="text-gray-300">Verify (Python/Rust only):</span> Run the bundled verifier binary for C2PA extraction before attesting.</li>
          </ol>
          <p class="text-xs text-gray-500 mt-2">Keypair format: Solana-compatible 64-byte JSON array (first 32 = secret key, last 32 = public key).</p>
        </div>
      </section>

      <!-- ============ INFRASTRUCTURE ============ -->
      <section v-show="activeSection === 'infra'" class="space-y-6">
        <h2 class="text-lg font-semibold border-b border-gray-800 pb-2">Infrastructure</h2>

        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div class="bg-gray-900 rounded-lg border border-green-800/40 p-5 space-y-3">
            <h3 class="text-sm font-semibold text-green-400">Local Dev</h3>
            <p class="text-xs text-gray-500"><span class="font-mono">./dev.sh</span> starts everything:</p>
            <ul class="text-xs text-gray-400 space-y-1 mt-2">
              <li>Builds Vue frontend (<span class="font-mono">npm run build</span>)</li>
              <li>Starts Docker Postgres on port 5432</li>
              <li>Builds Anchor program (if needed)</li>
              <li>Starts Solana test validator on port 8899</li>
              <li>Deploys program as BPF loader</li>
              <li>Starts Python API on port 3001</li>
            </ul>
          </div>
          <div class="bg-gray-900 rounded-lg border border-blue-800/40 p-5 space-y-3">
            <h3 class="text-sm font-semibold text-blue-400">AWS (Terraform ready)</h3>
            <p class="text-xs text-gray-500">~$25/mo estimated cost:</p>
            <ul class="text-xs text-gray-400 space-y-1 mt-2">
              <li>App Runner (1 vCPU, 2GB) — ~$7/mo</li>
              <li>RDS Postgres db.t4g.micro — ~$13/mo</li>
              <li>CloudFront distribution — free tier</li>
              <li>ECR repository — free tier</li>
              <li>VPC + private subnets + VPC connector</li>
            </ul>
          </div>
        </div>

        <div class="bg-gray-900 rounded-lg border border-gray-800 p-5 space-y-3">
          <h3 class="text-sm font-semibold text-gray-200">Docker</h3>
          <p class="text-xs text-gray-400">Single container bundles: Python API + Rust verifier binary + Vue SPA static files. Dockerfile at <span class="font-mono">docker/api-py.Dockerfile</span>.</p>
          <p class="text-xs text-gray-500 mt-1">Multi-stage build: Rust builder (compiles verifier) + Node builder (builds frontend) + Python runtime.</p>
        </div>

        <div class="bg-gray-900 rounded-lg border border-gray-800 p-5 space-y-3">
          <h3 class="text-sm font-semibold text-gray-200">Trust Lists</h3>
          <p class="text-xs text-gray-400">PEM certificate bundles in <span class="font-mono">data/trust/</span>:</p>
          <ul class="text-xs text-gray-500 space-y-1 mt-2">
            <li><span class="font-mono text-gray-400">official/</span> — C2PA trust list (Adobe, DigiCert, Google, Irdeto, SSL.com, etc.)</li>
            <li><span class="font-mono text-gray-400">curated/</span> — R3L additions (Adobe Firefly, OpenAI/Truepic, C2PA test certs)</li>
            <li><span class="font-mono text-gray-400">trust-list.pem</span> — concatenated bundle (auto-generated by <span class="font-mono">add-trust.sh</span>)</li>
          </ul>
          <p class="text-xs text-gray-500 mt-1">Trust bundle hash is SHA-256 of the concatenated PEM file, stored in every attestation for auditability.</p>
        </div>
      </section>

      <!-- ============ ROADMAP ============ -->
      <section v-show="activeSection === 'roadmap'" class="space-y-6">
        <h2 class="text-lg font-semibold border-b border-gray-800 pb-2">Roadmap</h2>

        <div class="space-y-3">
          <div class="bg-gray-900 rounded-lg border border-green-800/40 p-4 flex gap-4">
            <span class="text-green-400 text-xs font-semibold mt-0.5 shrink-0 w-16">DONE</span>
            <div>
              <p class="text-sm text-gray-200">C2PA extraction + trust list validation</p>
              <p class="text-xs text-gray-500">Rust verifier binary using c2pa-rs. Official + curated trust lists. Full metadata extraction.</p>
            </div>
          </div>
          <div class="bg-gray-900 rounded-lg border border-green-800/40 p-4 flex gap-4">
            <span class="text-green-400 text-xs font-semibold mt-0.5 shrink-0 w-16">DONE</span>
            <div>
              <p class="text-sm text-gray-200">On-chain attestation (Solana PDA)</p>
              <p class="text-xs text-gray-500">Anchor program with submit_attestation + submit_proof instructions. Ed25519 wallet verification via precompile.</p>
            </div>
          </div>
          <div class="bg-gray-900 rounded-lg border border-green-800/40 p-4 flex gap-4">
            <span class="text-green-400 text-xs font-semibold mt-0.5 shrink-0 w-16">DONE</span>
            <div>
              <p class="text-sm text-gray-200">Wallet identity binding (Ed25519 on-chain)</p>
              <p class="text-xs text-gray-500">Creator signs content hash, signature verified on-chain. Stored in PDA for independent re-verification.</p>
            </div>
          </div>
          <div class="bg-gray-900 rounded-lg border border-green-800/40 p-4 flex gap-4">
            <span class="text-green-400 text-xs font-semibold mt-0.5 shrink-0 w-16">DONE</span>
            <div>
              <p class="text-sm text-gray-200">Trust verdict API ("Truth Oracle")</p>
              <p class="text-xs text-gray-500">Structured query endpoint returning trusted/attested/unknown verdicts. Batch support. Designed for AI platform integration.</p>
            </div>
          </div>
          <div class="bg-gray-900 rounded-lg border border-green-800/40 p-4 flex gap-4">
            <span class="text-green-400 text-xs font-semibold mt-0.5 shrink-0 w-16">DONE</span>
            <div>
              <p class="text-sm text-gray-200">Edge node SDKs (Python, Rust, C)</p>
              <p class="text-xs text-gray-500">Three clients for different deployment targets. API key auth, wallet binding, C2PA extraction (Python/Rust).</p>
            </div>
          </div>

          <div class="bg-gray-900 rounded-lg border border-green-800/40 p-4 flex gap-4">
            <span class="text-green-400 text-xs font-semibold mt-0.5 shrink-0 w-16">DONE</span>
            <div>
              <p class="text-sm text-gray-200">Similarity search (TLSH + MobileCLIP2-S0)</p>
              <p class="text-xs text-gray-500">Byte-level TLSH hashing + cross-modal CLIP embeddings for images, videos, PDFs, and text files. pgvector cosine nearest-neighbor search. Stored per attestation.</p>
            </div>
          </div>

          <div class="bg-gray-900 rounded-lg border border-yellow-800/40 p-4 flex gap-4">
            <span class="text-yellow-400 text-xs font-semibold mt-0.5 shrink-0 w-16">READY</span>
            <div>
              <p class="text-sm text-gray-200">ZK Groth16 proof generation + on-chain verification</p>
              <p class="text-xs text-gray-500">SP1 prover guest program and Solana verifier instruction are built. Needs GPU deployment (g5.2xlarge, ~$1/hr) for real proof generation.</p>
            </div>
          </div>
          <div class="bg-gray-900 rounded-lg border border-yellow-800/40 p-4 flex gap-4">
            <span class="text-yellow-400 text-xs font-semibold mt-0.5 shrink-0 w-16">READY</span>
            <div>
              <p class="text-sm text-gray-200">Email domain verification</p>
              <p class="text-xs text-gray-500">API routes and UI built. Needs SMTP configuration for sending verification emails.</p>
            </div>
          </div>
          <div class="bg-gray-900 rounded-lg border border-yellow-800/40 p-4 flex gap-4">
            <span class="text-yellow-400 text-xs font-semibold mt-0.5 shrink-0 w-16">READY</span>
            <div>
              <p class="text-sm text-gray-200">AWS deployment</p>
              <p class="text-xs text-gray-500">Terraform config for App Runner + RDS + CloudFront. Docker multi-stage build. ~$25/mo.</p>
            </div>
          </div>

          <div class="bg-gray-900 rounded-lg border border-gray-800 p-4 flex gap-4 opacity-60">
            <span class="text-gray-500 text-xs font-semibold mt-0.5 shrink-0 w-16">NEXT</span>
            <div>
              <p class="text-sm text-gray-400">Mainnet deployment + real Solana transactions</p>
              <p class="text-xs text-gray-600">Currently running on localhost/devnet. Mainnet requires funded keypair + program deployment.</p>
            </div>
          </div>
          <div class="bg-gray-900 rounded-lg border border-gray-800 p-4 flex gap-4 opacity-60">
            <span class="text-gray-500 text-xs font-semibold mt-0.5 shrink-0 w-16">NEXT</span>
            <div>
              <p class="text-sm text-gray-400">DID / ENS identity layer</p>
              <p class="text-xs text-gray-600">Decentralized identity beyond wallet pubkeys. Organizational verification.</p>
            </div>
          </div>
          <div class="bg-gray-900 rounded-lg border border-gray-800 p-4 flex gap-4 opacity-60">
            <span class="text-gray-500 text-xs font-semibold mt-0.5 shrink-0 w-16">FUTURE</span>
            <div>
              <p class="text-sm text-gray-400">Marketplace, licensing, royalty tracking</p>
              <p class="text-xs text-gray-600">Content marketplace with creator royalties and licensing infrastructure.</p>
            </div>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>
