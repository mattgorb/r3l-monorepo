<template>
  <div class="space-y-16">
    <!-- Hero -->
    <div class="space-y-5">
      <h1 class="text-4xl font-bold">The Trust Layer for Digital Content</h1>
      <div class="space-y-2 max-w-2xl">
        <p class="text-lg text-gray-400">Hash any data, link it to a verified identity, and record it permanently on Solana.</p>
        <p class="text-lg text-gray-400">Query any content and get an instant trust verdict.</p>
        <p class="text-lg text-gray-400">Built for developers, organizations, and AI agents.</p>
      </div>
    </div>

    <!-- How it works -->
    <div class="space-y-4">
      <h2 class="text-xl font-semibold">How It Works</h2>
      <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
        <router-link to="/account" class="bg-gray-900 rounded-lg border border-gray-800 p-5 space-y-2 hover:border-gray-600 transition-colors block">
          <p class="text-sm font-semibold text-gray-200">Verify Identity <span class="text-gray-500 font-normal">(optional)</span></p>
          <p class="text-xs text-gray-500">Register with your wallet, email, or organization. Wallet signatures are verified on-chain via Ed25519 precompile.</p>
        </router-link>
        <router-link to="/attest" class="bg-gray-900 rounded-lg border border-gray-800 p-5 space-y-2 hover:border-gray-600 transition-colors block">
          <p class="text-sm font-semibold text-gray-200">Attest</p>
          <p class="text-xs text-gray-500">Submit a file, URL, or text. R3L computes the SHA-256 hash and writes the attestation to a Solana PDA — permanent and tamper-proof.</p>
        </router-link>
        <router-link to="/search" class="bg-gray-900 rounded-lg border border-gray-800 p-5 space-y-2 hover:border-gray-600 transition-colors block">
          <p class="text-sm font-semibold text-gray-200">Query</p>
          <p class="text-xs text-gray-500">Look up any content hash for a trust verdict, or search by file to find similar attested content via CLIP semantic similarity.</p>
        </router-link>
      </div>
    </div>

    <!-- What you can attest -->
    <div class="space-y-4">
      <h2 class="text-xl font-semibold">Attest Any Data</h2>
      <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
        <div class="bg-gray-900 rounded-lg border border-gray-800 p-5 space-y-2">
          <p class="text-sm font-semibold text-gray-200">Files</p>
          <p class="text-xs text-gray-500">Images, video, audio, PDFs, CSVs, and more. SHA-256 content hash with optional C2PA metadata extraction for media files.</p>
        </div>
        <div class="bg-gray-900 rounded-lg border border-gray-800 p-5 space-y-2">
          <p class="text-sm font-semibold text-gray-200">URLs</p>
          <p class="text-xs text-gray-500">Snapshot any web page. Fetches the content, hashes it, and creates a timestamped attestation of what was there.</p>
        </div>
        <div class="bg-gray-900 rounded-lg border border-gray-800 p-5 space-y-2">
          <p class="text-sm font-semibold text-gray-200">Text</p>
          <p class="text-xs text-gray-500">JSON, CSV data, API responses, articles, or any plaintext. Hash and record content that doesn't live in a file.</p>
        </div>
      </div>
    </div>

    <!-- For developers -->
    <div class="space-y-4">
      <h2 class="text-xl font-semibold">For Developers</h2>
      <div class="bg-gray-900 rounded-lg border border-gray-800 p-5 space-y-4">
        <div class="space-y-2">
          <p class="text-sm font-semibold text-gray-200">REST API</p>
          <p class="text-xs text-gray-500">Register, attest content, query verdicts, and search for similar data. Authenticate with an API key and start attesting in minutes.</p>
        </div>
        <div class="space-y-2">
          <p class="text-sm font-semibold text-gray-200">Batch Operations</p>
          <p class="text-xs text-gray-500">Attest multiple files in a single request. Query up to 50 content hashes at once for high-throughput workflows.</p>
        </div>
        <div class="space-y-2">
          <p class="text-sm font-semibold text-gray-200">Edge Nodes</p>
          <p class="text-xs text-gray-500">Run verification locally with the <span class="font-mono text-gray-400">r3l-edge</span> Python SDK. Compute hashes, CLIP embeddings, and C2PA extraction on your own infrastructure before submitting to R3L.</p>
        </div>
        <router-link to="/docs" class="inline-block text-sm text-blue-400 hover:text-blue-300 transition-colors">
          View API docs &rarr;
        </router-link>
      </div>
    </div>

    <!-- For AI agents -->
    <div class="space-y-6">
      <h2 class="text-xl font-semibold">For AI Agents</h2>
      <p class="text-sm text-gray-500">R3L returns machine-readable trust verdicts over API, so agents can verify data and find attested sources without human intervention.</p>

      <!-- Diagram 1: Verify Unknown Data -->
      <div class="space-y-3">
        <p class="text-sm font-semibold text-gray-300">Verify Unknown Data</p>
        <p class="text-xs text-gray-500">Agent encounters data it hasn't seen before. Hash it and check R3L to see if a trusted identity has already attested it.</p>
        <div class="bg-gray-900 rounded-lg border border-gray-800 p-6 overflow-x-auto">
          <div class="flex items-center gap-3 min-w-[600px]">
            <!-- Step: Unknown Data -->
            <div class="shrink-0 bg-gray-800 rounded-lg px-4 py-3 text-center">
              <p class="text-xs font-semibold text-gray-300">Unknown Data</p>
              <p class="text-[10px] text-gray-500">file, URL, or text</p>
            </div>
            <div class="text-gray-600 shrink-0">&rarr;</div>
            <!-- Step: Hash + Query -->
            <div class="shrink-0 bg-blue-900/40 border border-blue-800/50 rounded-lg px-4 py-3 text-center">
              <p class="text-xs font-semibold text-blue-400">R3L Query</p>
              <p class="text-[10px] text-gray-500">hash &rarr; /api/v1/query/{hash}</p>
            </div>
            <div class="text-gray-600 shrink-0">&rarr;</div>
            <!-- Step: Decision -->
            <div class="shrink-0 flex flex-col gap-2">
              <div class="bg-green-900/30 border border-green-800/50 rounded-lg px-4 py-2 text-center">
                <p class="text-[10px] font-semibold text-green-400">trusted / attested</p>
                <p class="text-[10px] text-gray-500">act on data with confidence</p>
              </div>
              <div class="bg-red-900/30 border border-red-800/50 rounded-lg px-4 py-2 text-center">
                <p class="text-[10px] font-semibold text-red-400">unknown</p>
                <p class="text-[10px] text-gray-500">reject, flag, or request attestation</p>
              </div>
            </div>
          </div>
        </div>
        <pre class="bg-gray-950 rounded border border-gray-800 px-4 py-3 text-xs font-mono text-gray-400 overflow-x-auto">curl https://d2ok2lga3g52rl.cloudfront.net/api/v1/query/a1b2c3...
# &rarr; { "verdict": "trusted", "identity": { "org": "reuters.com" }, "proof": { "on_chain": true } }</pre>
      </div>

      <!-- Diagram 2: RAG with Attested Sources -->
      <div class="space-y-3">
        <p class="text-sm font-semibold text-gray-300">RAG with Attested Sources</p>
        <p class="text-xs text-gray-500">Use R3L similarity search to find attested content relevant to a query. Ground agent responses in verified, on-chain data.</p>
        <div class="bg-gray-900 rounded-lg border border-gray-800 p-6 overflow-x-auto">
          <div class="flex items-center gap-3 min-w-[600px]">
            <!-- Step: Agent Query -->
            <div class="shrink-0 bg-gray-800 rounded-lg px-4 py-3 text-center">
              <p class="text-xs font-semibold text-gray-300">Agent Query</p>
              <p class="text-[10px] text-gray-500">file or content hash</p>
            </div>
            <div class="text-gray-600 shrink-0">&rarr;</div>
            <!-- Step: Similarity Search -->
            <div class="shrink-0 bg-blue-900/40 border border-blue-800/50 rounded-lg px-4 py-3 text-center">
              <p class="text-xs font-semibold text-blue-400">R3L Similar</p>
              <p class="text-[10px] text-gray-500">/api/v1/similar</p>
            </div>
            <div class="text-gray-600 shrink-0">&rarr;</div>
            <!-- Step: Attested Matches -->
            <div class="shrink-0 bg-gray-800 rounded-lg px-4 py-3 text-center">
              <p class="text-xs font-semibold text-gray-300">Attested Matches</p>
              <p class="text-[10px] text-gray-500">CLIP + TLSH ranked</p>
            </div>
            <div class="text-gray-600 shrink-0">&rarr;</div>
            <!-- Step: Grounded Response -->
            <div class="shrink-0 bg-green-900/30 border border-green-800/50 rounded-lg px-4 py-3 text-center">
              <p class="text-xs font-semibold text-green-400">Grounded Response</p>
              <p class="text-[10px] text-gray-500">cite verified sources</p>
            </div>
          </div>
        </div>
        <pre class="bg-gray-950 rounded border border-gray-800 px-4 py-3 text-xs font-mono text-gray-400 overflow-x-auto">curl -X POST https://d2ok2lga3g52rl.cloudfront.net/api/v1/similar -F "file=@query.jpg"
# &rarr; [{ "match": "exact", "content_hash": "ab12...", "identity": { "org": "ap.org" }, "clip_similarity": 0.97 }]</pre>
      </div>

      <!-- Diagram 3: Attest Agent Outputs -->
      <div class="space-y-3">
        <p class="text-sm font-semibold text-gray-300">Attest Agent Outputs</p>
        <p class="text-xs text-gray-500">Agents attest data they produce so downstream consumers &mdash; other agents or humans &mdash; can verify who created it, when, and under which organization.</p>
        <div class="bg-gray-900 rounded-lg border border-gray-800 p-6 overflow-x-auto">
          <div class="flex items-center gap-3 min-w-[600px]">
            <!-- Step: Agent produces output -->
            <div class="shrink-0 bg-gray-800 rounded-lg px-4 py-3 text-center">
              <p class="text-xs font-semibold text-gray-300">Agent Output</p>
              <p class="text-[10px] text-gray-500">report, analysis, or dataset</p>
            </div>
            <div class="text-gray-600 shrink-0">&rarr;</div>
            <!-- Step: Attest to R3L -->
            <div class="shrink-0 bg-blue-900/40 border border-blue-800/50 rounded-lg px-4 py-3 text-center">
              <p class="text-xs font-semibold text-blue-400">R3L Attest</p>
              <p class="text-[10px] text-gray-500">/api/v1/attest-content</p>
            </div>
            <div class="text-gray-600 shrink-0">&rarr;</div>
            <!-- Step: On-chain record -->
            <div class="shrink-0 bg-gray-800 rounded-lg px-4 py-3 text-center">
              <p class="text-xs font-semibold text-gray-300">On-Chain Record</p>
              <p class="text-[10px] text-gray-500">hash + identity + timestamp</p>
            </div>
            <div class="text-gray-600 shrink-0">&rarr;</div>
            <!-- Step: Downstream verification -->
            <div class="shrink-0 bg-green-900/30 border border-green-800/50 rounded-lg px-4 py-3 text-center">
              <p class="text-xs font-semibold text-green-400">Verifiable by Anyone</p>
              <p class="text-[10px] text-gray-500">other agents, auditors, users</p>
            </div>
          </div>
        </div>
        <pre class="bg-gray-950 rounded border border-gray-800 px-4 py-3 text-xs font-mono text-gray-400 overflow-x-auto">curl -X POST https://d2ok2lga3g52rl.cloudfront.net/api/v1/attest-content \
  -H "X-API-Key: YOUR_KEY" \
  -F "file=@report.pdf"
# &rarr; { "content_hash": "e4f5...", "tx_signature": "3xK9...", "pda": "7mR2..." }</pre>
      </div>
    </div>

    <!-- Identity Verification -->
    <div class="space-y-4">
      <h2 class="text-xl font-semibold">Identity Verification</h2>
      <p class="text-sm text-gray-500">Link attestations to verified identities. Multiple identity types can be combined on a single account.</p>
      <div class="bg-gray-900 rounded-lg border border-gray-800 divide-y divide-gray-800">
        <div class="flex items-start gap-4 px-5 py-4">
          <span class="text-purple-400 text-sm mt-0.5 shrink-0 w-5 text-center">&#9919;</span>
          <div>
            <p class="text-sm font-medium text-gray-200">Wallet Identity</p>
            <p class="text-xs text-gray-500">Solana wallet signs the content hash. Ed25519 signature verified on-chain via precompile instruction.</p>
          </div>
        </div>
        <div class="flex items-start gap-4 px-5 py-4">
          <span class="text-yellow-400 text-sm mt-0.5 shrink-0 w-5 text-center">&#9993;</span>
          <div>
            <p class="text-sm font-medium text-gray-200">Email Domain</p>
            <p class="text-xs text-gray-500">Verified email address linked to the attestation. Domain recorded on-chain (e.g. @nytimes.com).</p>
          </div>
        </div>
        <div class="flex items-start gap-4 px-5 py-4">
          <span class="text-cyan-400 text-sm mt-0.5 shrink-0 w-5 text-center">&#9737;</span>
          <div>
            <p class="text-sm font-medium text-gray-200">Organization</p>
            <p class="text-xs text-gray-500">Verified organization domain via DNS TXT record or admin email. Proves institutional affiliation with DID issuance.</p>
          </div>
        </div>
      </div>
    </div>

    <!-- Content Verification -->
    <div class="space-y-4">
      <h2 class="text-xl font-semibold">Content Verification</h2>
      <p class="text-sm text-gray-500">Every attestation records multiple content signals on-chain for independent verification.</p>
      <div class="bg-gray-900 rounded-lg border border-gray-800 divide-y divide-gray-800">
        <div class="flex items-start gap-4 px-5 py-4">
          <span class="text-blue-400 text-sm mt-0.5 shrink-0 w-5 text-center">#</span>
          <div>
            <p class="text-sm font-medium text-gray-200">Content Hash</p>
            <p class="text-xs text-gray-500">SHA-256 hash of raw content. The universal identifier for every attestation.</p>
          </div>
        </div>
        <div class="flex items-start gap-4 px-5 py-4">
          <span class="text-green-400 text-sm mt-0.5 shrink-0 w-5 text-center">&#10003;</span>
          <div>
            <p class="text-sm font-medium text-gray-200">C2PA Metadata</p>
            <p class="text-xs text-gray-500">If present in media files, R3L extracts embedded metadata: issuer, source type (camera vs AI), signing time, and trust list status.</p>
          </div>
        </div>
        <div class="flex items-start gap-4 px-5 py-4">
          <span class="text-orange-400 text-sm mt-0.5 shrink-0 w-5 text-center">&#8776;</span>
          <div>
            <p class="text-sm font-medium text-gray-200">Similarity Detection</p>
            <p class="text-xs text-gray-500">TLSH fuzzy hashing and CLIP neural embeddings detect near-duplicates and visually similar content across all attestations.</p>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
