<script setup lang="ts">
import { ref } from 'vue'

const activeSection = ref('getting-started')
const copiedField = ref<string | null>(null)

const sections = [
  { id: 'getting-started', label: 'Getting Started' },
  { id: 'auth', label: 'Authentication' },
  { id: 'register', label: 'Register' },
  { id: 'verify-identity', label: 'Verify Identity' },
  { id: 'content', label: 'Attest Content' },
  { id: 'lookup', label: 'Lookup' },
  { id: 'list', label: 'List Attestations' },
  { id: 'query', label: 'Query Verdict' },
  { id: 'similar', label: 'Similarity Search' },
  { id: 'account', label: 'Account' },
]

const baseUrl = window.location.origin

const curls = {
  qs1: `curl -X POST ${baseUrl}/api/v1/register -H 'Content-Type: application/json' -d '{"email": "you@example.com", "name": "My App"}'`,
  qs2: `curl -X POST ${baseUrl}/api/v1/attest-content -H 'X-API-Key: YOUR_API_KEY' -F 'file=@photo.jpg'`,
  qs3: `curl ${baseUrl}/api/v1/query/CONTENT_HASH`,
  reg1: `curl -X POST ${baseUrl}/api/v1/register -H 'Content-Type: application/json' -d '{"email": "dev@example.com", "name": "My App"}'`,
  // content endpoints
  cf1: `curl -X POST ${baseUrl}/api/v1/attest-content -H 'X-API-Key: YOUR_API_KEY' -F 'file=@photo.jpg'`,
  cu1: `curl -X POST ${baseUrl}/api/v1/attest-content -H 'X-API-Key: YOUR_API_KEY' -F 'url=https://example.com/article'`,
  ct1: `curl -X POST ${baseUrl}/api/v1/attest-content -H 'X-API-Key: YOUR_API_KEY' -F 'text=Hello, world!' -F 'title=Test'`,
  cb1: `curl -X POST ${baseUrl}/api/v1/attest-content/batch -H 'X-API-Key: YOUR_API_KEY' -F 'file=@photo.jpg' -F 'url=https://example.com/article' -F 'text=Hello, world!'`,
  // lookup / query / similar
  lk1: `curl ${baseUrl}/api/attestation/CONTENT_HASH`,
  qv1: `curl ${baseUrl}/api/v1/query/CONTENT_HASH`,
  qb1: `curl -X POST ${baseUrl}/api/v1/query/batch -H 'Content-Type: application/json' -d '["hash1...", "hash2..."]'`,
  sim1: `curl -X POST ${baseUrl}/api/v1/similar -F 'file=@photo.jpg'`,
  sim2: `curl ${baseUrl}/api/v1/similar/CONTENT_HASH`,
  // verify identity
  vi1: `curl -X POST ${baseUrl}/api/v1/verify-identity -H 'Content-Type: application/json' -H 'X-API-Key: YOUR_API_KEY' -d '{"email": "you@example.com"}'`,
  vi2: `curl -X POST ${baseUrl}/api/v1/verify-identity -H 'Content-Type: application/json' -H 'X-API-Key: YOUR_API_KEY' -d '{"email": "you@example.com", "code": "123456"}'`,
  vi3: `curl -X POST ${baseUrl}/api/v1/verify-identity -H 'Content-Type: application/json' -H 'X-API-Key: YOUR_API_KEY' -d '{"wallet_pubkey": "BASE58_PUBKEY", "wallet_message": "R3L-verify:my-app", "wallet_signature": "BASE58_SIG"}'`,
  vi4: `curl -X POST ${baseUrl}/api/v1/verify-identity -H 'Content-Type: application/json' -H 'X-API-Key: YOUR_API_KEY' -d '{"org_domain": "example.com"}'`,
  vi5: `curl -X POST ${baseUrl}/api/v1/verify-identity -H 'Content-Type: application/json' -H 'X-API-Key: YOUR_API_KEY' -d '{"email": "you@example.com", "wallet_pubkey": "BASE58_PUBKEY", "wallet_message": "R3L-verify:my-app", "wallet_signature": "BASE58_SIG", "org_domain": "example.com"}'`,
  // account
  me1: `curl ${baseUrl}/api/v1/me -H 'X-API-Key: YOUR_API_KEY'`,
  priv1: `curl -X PATCH ${baseUrl}/api/auth/me/privacy -H 'Content-Type: application/json' -H 'X-API-Key: YOUR_API_KEY' -d '{"privacy_mode": true}'`,
}

async function copyToClipboard(value: string, field: string) {
  try {
    await navigator.clipboard.writeText(value)
    copiedField.value = field
    setTimeout(() => { copiedField.value = null }, 1500)
  } catch { /* no-op */ }
}
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
        <h1 class="text-2xl font-bold">API Reference</h1>
        <p class="text-sm text-gray-500 mt-1">Everything you need to integrate with R3L programmatically.</p>
      </div>

      <!-- Mobile section selector -->
      <select
        v-model="activeSection"
        class="md:hidden w-full bg-gray-900 border border-gray-700 rounded-lg px-3 py-2 text-sm"
      >
        <option v-for="s in sections" :key="s.id" :value="s.id">{{ s.label }}</option>
      </select>

      <!-- ============ GETTING STARTED ============ -->
      <section v-show="activeSection === 'getting-started'" class="space-y-6">
        <h2 class="text-lg font-semibold border-b border-gray-800 pb-2">Getting Started</h2>

        <div class="space-y-4">
          <p class="text-sm text-gray-400">R3L provides on-chain attestation for any digital content. Register an account, verify your identity, and attest content in a few API calls.</p>

          <div class="bg-gray-900 rounded-lg border border-gray-800 p-5 space-y-4">
            <h3 class="text-sm font-semibold text-gray-200">Quick Start</h3>
            <div class="space-y-4">
              <div class="flex gap-3 items-start">
                <span class="bg-blue-900/60 text-blue-400 text-xs font-mono px-2 py-0.5 rounded shrink-0 mt-0.5">1</span>
                <div>
                  <p class="text-sm text-gray-300">Register and get an API key</p>
                  <div class="mt-2 relative">
                    <pre class="bg-gray-950 rounded border border-gray-800 px-4 py-3 text-xs font-mono text-gray-400 overflow-x-auto">curl -X POST {{ baseUrl }}/api/v1/register \
  -H "Content-Type: application/json" \
  -d '{"email": "you@example.com", "name": "My App"}'</pre>
                    <button @click="copyToClipboard(curls.qs1, 'qs1')" class="absolute top-2 right-2 text-gray-600 hover:text-gray-400 cursor-pointer text-xs">
                      {{ copiedField === 'qs1' ? 'Copied!' : 'Copy' }}
                    </button>
                  </div>
                  <p class="text-xs text-gray-600 mt-1">Returns <span class="font-mono text-gray-500">{"api_key": "r3l_...", ...}</span></p>
                </div>
              </div>
              <div class="flex gap-3 items-start">
                <span class="bg-blue-900/60 text-blue-400 text-xs font-mono px-2 py-0.5 rounded shrink-0 mt-0.5">2</span>
                <div>
                  <p class="text-sm text-gray-300">Verify your identity <span class="text-gray-600">(optional)</span></p>
                  <p class="text-xs text-gray-500 mt-1 mb-2">Use <span class="font-mono text-gray-400">/verify-identity</span> to link email, wallet, and/or org — any combination in one call.</p>

                  <p class="text-xs text-gray-400 mt-3 mb-1 font-medium">Confirm the email code:</p>
                  <div class="relative">
                    <pre class="bg-gray-950 rounded border border-gray-800 px-4 py-3 text-xs font-mono text-gray-400 overflow-x-auto">curl -X POST {{ baseUrl }}/api/v1/verify-identity \
  -H "Content-Type: application/json" \
  -H "X-API-Key: YOUR_API_KEY" \
  -d '{"email": "you@example.com", "code": "123456"}'</pre>
                    <button @click="copyToClipboard(curls.vi2, 'vi2qs')" class="absolute top-2 right-2 text-gray-600 hover:text-gray-400 cursor-pointer text-xs">
                      {{ copiedField === 'vi2qs' ? 'Copied!' : 'Copy' }}
                    </button>
                  </div>
                  <p class="text-xs text-gray-600 mt-1">Returns <span class="font-mono text-gray-500">{"email_status": "verified"}</span> — all identities now linked.</p>
                  <p class="text-xs text-gray-600 mt-2">See <button @click="activeSection = 'verify-identity'" class="text-blue-400 hover:underline cursor-pointer">Verify Identity</button> for full details.</p>
                </div>
              </div>
              <div class="flex gap-3 items-start">
                <span class="bg-blue-900/60 text-blue-400 text-xs font-mono px-2 py-0.5 rounded shrink-0 mt-0.5">3</span>
                <div>
                  <p class="text-sm text-gray-300">Verify and attest content</p>
                  <p class="text-xs text-gray-500 mt-1 mb-2">One endpoint for file, URL, or text. Use <span class="font-mono text-gray-400">/attest-content</span> for one at a time, or <span class="font-mono text-gray-400">/attest-content/batch</span> for multiple.</p>

                  <p class="text-xs text-gray-400 mt-3 mb-1 font-medium">Attest a file:</p>
                  <div class="relative">
                    <pre class="bg-gray-950 rounded border border-gray-800 px-4 py-3 text-xs font-mono text-gray-400 overflow-x-auto">curl -X POST {{ baseUrl }}/api/v1/attest-content \
  -H "X-API-Key: YOUR_API_KEY" \
  -F "file=@photo.jpg"</pre>
                    <button @click="copyToClipboard(curls.cf1, 'cf1qs')" class="absolute top-2 right-2 text-gray-600 hover:text-gray-400 cursor-pointer text-xs">
                      {{ copiedField === 'cf1qs' ? 'Copied!' : 'Copy' }}
                    </button>
                  </div>

                  <p class="text-xs text-gray-400 mt-3 mb-1 font-medium">Attest a URL:</p>
                  <div class="relative">
                    <pre class="bg-gray-950 rounded border border-gray-800 px-4 py-3 text-xs font-mono text-gray-400 overflow-x-auto">curl -X POST {{ baseUrl }}/api/v1/attest-content \
  -H "X-API-Key: YOUR_API_KEY" \
  -F "url=https://example.com/article"</pre>
                    <button @click="copyToClipboard(curls.cu1, 'cu1qs')" class="absolute top-2 right-2 text-gray-600 hover:text-gray-400 cursor-pointer text-xs">
                      {{ copiedField === 'cu1qs' ? 'Copied!' : 'Copy' }}
                    </button>
                  </div>

                  <p class="text-xs text-gray-400 mt-3 mb-1 font-medium">Attest text:</p>
                  <div class="relative">
                    <pre class="bg-gray-950 rounded border border-gray-800 px-4 py-3 text-xs font-mono text-gray-400 overflow-x-auto">curl -X POST {{ baseUrl }}/api/v1/attest-content \
  -H "X-API-Key: YOUR_API_KEY" \
  -F "text=Hello, world!" \
  -F "title=My first attestation"</pre>
                    <button @click="copyToClipboard(curls.ct1, 'ct1qs')" class="absolute top-2 right-2 text-gray-600 hover:text-gray-400 cursor-pointer text-xs">
                      {{ copiedField === 'ct1qs' ? 'Copied!' : 'Copy' }}
                    </button>
                  </div>

                  <p class="text-xs text-gray-600 mt-2">Files are verified for C2PA metadata. All content is hashed, stored, and attested on Solana.</p>
                  <p class="text-xs text-gray-600 mt-2">See <button @click="activeSection = 'content'" class="text-blue-400 hover:underline cursor-pointer">Attest Content</button> for batch examples and all options.</p>
                </div>
              </div>
              <div class="flex gap-3 items-start">
                <span class="bg-blue-900/60 text-blue-400 text-xs font-mono px-2 py-0.5 rounded shrink-0 mt-0.5">4</span>
                <div>
                  <p class="text-sm text-gray-300">Look up the attestation</p>
                  <div class="mt-2 relative">
                    <pre class="bg-gray-950 rounded border border-gray-800 px-4 py-3 text-xs font-mono text-gray-400 overflow-x-auto">curl {{ baseUrl }}/api/v1/query/CONTENT_HASH</pre>
                    <button @click="copyToClipboard(curls.qs3, 'qs3')" class="absolute top-2 right-2 text-gray-600 hover:text-gray-400 cursor-pointer text-xs">
                      {{ copiedField === 'qs3' ? 'Copied!' : 'Copy' }}
                    </button>
                  </div>
                  <p class="text-xs text-gray-600 mt-1">Returns a structured trust verdict: <span class="font-mono text-green-400">trusted</span>, <span class="font-mono text-blue-400">attested</span>, or <span class="font-mono text-gray-400">unknown</span>.</p>
                </div>
              </div>
            </div>
          </div>

          <div class="bg-gray-900 rounded-lg border border-gray-800 p-5 space-y-3">
            <h3 class="text-sm font-semibold text-gray-200">Base URL</h3>
            <p class="text-xs text-gray-400">All API endpoints are relative to:</p>
            <code class="text-sm font-mono text-blue-400">{{ baseUrl }}/api</code>
          </div>
        </div>
      </section>

      <!-- ============ AUTHENTICATION ============ -->
      <section v-show="activeSection === 'auth'" class="space-y-6">
        <h2 class="text-lg font-semibold border-b border-gray-800 pb-2">Authentication</h2>

        <div class="space-y-4">
          <p class="text-sm text-gray-400">
            Authenticated requests use the <span class="font-mono text-gray-300">X-API-Key</span> header.
            Get your key from the <span class="font-mono text-gray-300">/api/v1/register</span> endpoint or the Account page.
          </p>

          <div class="bg-gray-900 rounded-lg border border-gray-800 p-5 space-y-3">
            <h3 class="text-sm font-semibold text-gray-200">Header Format</h3>
            <pre class="bg-gray-950 rounded border border-gray-800 px-4 py-3 text-xs font-mono text-gray-400">X-API-Key: r3l_your_api_key_here</pre>
          </div>

          <div class="bg-gray-900 rounded-lg border border-gray-800 p-5 space-y-3">
            <h3 class="text-sm font-semibold text-gray-200">Auth Requirements</h3>
            <div class="overflow-x-auto">
              <table class="w-full text-xs">
                <thead>
                  <tr class="text-left text-gray-500 border-b border-gray-800">
                    <th class="pb-2 pr-4">Endpoint</th>
                    <th class="pb-2 pr-4">Auth</th>
                    <th class="pb-2">Notes</th>
                  </tr>
                </thead>
                <tbody class="text-gray-400">
                  <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono">/api/v1/register</td><td class="pr-4 text-gray-500">None</td><td>Creates account</td></tr>
                  <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono">/api/v1/verify-identity</td><td class="pr-4 text-green-400">Required</td><td>Any combination of identity types</td></tr>
                  <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono">/api/v1/attest-content</td><td class="pr-4 text-yellow-400">Optional</td><td>One content type per call (file, URL, or text)</td></tr>
                  <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono">/api/v1/attest-content/batch</td><td class="pr-4 text-yellow-400">Optional</td><td>Multiple content types at once</td></tr>
                  <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono">/api/v1/me</td><td class="pr-4 text-green-400">Required</td><td>Account info</td></tr>
                  <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono">/api/attestation/{hash}</td><td class="pr-4 text-gray-500">None</td><td>Public lookup</td></tr>
                  <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono">/api/v1/query/{hash}</td><td class="pr-4 text-gray-500">None</td><td>Public verdict</td></tr>
                  <tr><td class="py-2 pr-4 font-mono">/api/v1/similar</td><td class="pr-4 text-gray-500">None</td><td>Public search</td></tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>
      </section>

      <!-- ============ REGISTER ============ -->
      <section v-show="activeSection === 'register'" class="space-y-6">
        <h2 class="text-lg font-semibold border-b border-gray-800 pb-2">Register</h2>

        <!-- POST /api/v1/register -->
        <div class="bg-gray-900 rounded-lg border border-blue-800/40 overflow-hidden">
          <div class="flex items-center gap-3 px-5 py-3 border-b border-gray-800">
            <span class="bg-blue-900/60 text-blue-400 text-xs font-mono px-2 py-0.5 rounded">POST</span>
            <span class="font-mono text-sm text-gray-200">/api/v1/register</span>
          </div>
          <div class="px-5 py-4 space-y-4">
            <p class="text-sm text-gray-400">Create a developer account. Provide any combination of email, wallet, or organization. Returns an API key immediately.</p>

            <div>
              <h4 class="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Request Body <span class="text-gray-600 normal-case">(JSON)</span></h4>
              <div class="overflow-x-auto">
                <table class="w-full text-xs">
                  <thead>
                    <tr class="text-left text-gray-500 border-b border-gray-800">
                      <th class="pb-2 pr-4">Field</th>
                      <th class="pb-2 pr-4">Type</th>
                      <th class="pb-2 pr-4">Required</th>
                      <th class="pb-2">Description</th>
                    </tr>
                  </thead>
                  <tbody class="text-gray-400">
                    <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono text-gray-300">name</td><td class="pr-4">string</td><td class="pr-4 text-gray-600">No</td><td>Display name for your account</td></tr>
                    <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono text-gray-300">email</td><td class="pr-4">string</td><td class="pr-4 text-gray-600">No*</td><td>Email address</td></tr>
                    <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono text-gray-300">wallet_pubkey</td><td class="pr-4">string</td><td class="pr-4 text-gray-600">No*</td><td>Solana pubkey (base58)</td></tr>
                    <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono text-gray-300">wallet_message</td><td class="pr-4">string</td><td class="pr-4 text-gray-600">If wallet</td><td>Must start with <span class="font-mono">"R3L-register:"</span></td></tr>
                    <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono text-gray-300">wallet_signature</td><td class="pr-4">string</td><td class="pr-4 text-gray-600">If wallet</td><td>Ed25519 signature (base58)</td></tr>
                    <tr><td class="py-2 pr-4 font-mono text-gray-300">org_domain</td><td class="pr-4">string</td><td class="pr-4 text-gray-600">No*</td><td>Link to a verified organization</td></tr>
                  </tbody>
                </table>
              </div>
              <p class="text-xs text-gray-600 mt-2">*At least one of <span class="font-mono">email</span>, <span class="font-mono">wallet_pubkey</span>, or <span class="font-mono">org_domain</span> is required.</p>
            </div>

            <div>
              <h4 class="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Response</h4>
              <pre class="bg-gray-950 rounded border border-gray-800 px-4 py-3 text-xs font-mono text-gray-400 overflow-x-auto">{
  "api_key": "r3l_abc123...",
  "name": "My App",
  "email": "dev@example.com",
  "wallet_pubkey": "base58...",
  "org_domain": "example.com",
  "existing": false
}</pre>
            </div>

            <div>
              <h4 class="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Example</h4>
              <div class="relative">
                <pre class="bg-gray-950 rounded border border-gray-800 px-4 py-3 text-xs font-mono text-gray-400 overflow-x-auto">curl -X POST {{ baseUrl }}/api/v1/register \
  -H "Content-Type: application/json" \
  -d '{"email": "dev@example.com", "name": "My App"}'</pre>
                <button @click="copyToClipboard(curls.reg1, 'reg1')" class="absolute top-2 right-2 text-gray-600 hover:text-gray-400 cursor-pointer text-xs">
                  {{ copiedField === 'reg1' ? 'Copied!' : 'Copy' }}
                </button>
              </div>
            </div>
          </div>
        </div>
      </section>

      <!-- ============ VERIFY IDENTITY ============ -->
      <section v-show="activeSection === 'verify-identity'" class="space-y-6">
        <h2 class="text-lg font-semibold border-b border-gray-800 pb-2">Verify Identity</h2>

        <p class="text-sm text-gray-400">Link identities to your account. Use <span class="font-mono text-gray-300">/verify-identity</span> with any combination of email, wallet, and org in one call. Requires the <span class="font-mono text-gray-300">X-API-Key</span> header.</p>

        <!-- POST /api/v1/verify-identity -->
        <div class="bg-gray-900 rounded-lg border border-blue-800/40 overflow-hidden">
          <div class="flex items-center gap-3 px-5 py-3 border-b border-gray-800">
            <span class="bg-blue-900/60 text-blue-400 text-xs font-mono px-2 py-0.5 rounded">POST</span>
            <span class="font-mono text-sm text-gray-200">/api/v1/verify-identity</span>
            <span class="text-xs text-green-500">auth required</span>
          </div>
          <div class="px-5 py-4 space-y-4">
            <p class="text-sm text-gray-400">Verify and link identities to your account. Send any combination of email, wallet, and org in a single request. Wallet and org verify instantly; email requires a two-step code flow.</p>

            <div>
              <h4 class="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Request Body <span class="text-gray-600 normal-case">(JSON)</span></h4>
              <div class="overflow-x-auto">
                <table class="w-full text-xs">
                  <thead>
                    <tr class="text-left text-gray-500 border-b border-gray-800">
                      <th class="pb-2 pr-4">Field</th>
                      <th class="pb-2 pr-4">Type</th>
                      <th class="pb-2 pr-4">Required</th>
                      <th class="pb-2">Description</th>
                    </tr>
                  </thead>
                  <tbody class="text-gray-400">
                    <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono text-gray-300">email</td><td class="pr-4">string</td><td class="pr-4 text-gray-600">No*</td><td>Email to verify. Without <span class="font-mono">code</span>, sends a 6-digit code. With <span class="font-mono">code</span>, confirms and links.</td></tr>
                    <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono text-gray-300">code</td><td class="pr-4">string</td><td class="pr-4 text-gray-600">No</td><td>6-digit code from your inbox (use with <span class="font-mono">email</span>)</td></tr>
                    <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono text-gray-300">wallet_pubkey</td><td class="pr-4">string</td><td class="pr-4 text-gray-600">No*</td><td>Solana pubkey (base58)</td></tr>
                    <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono text-gray-300">wallet_message</td><td class="pr-4">string</td><td class="pr-4 text-gray-600">If wallet</td><td>Must start with <span class="font-mono">"R3L-verify:"</span></td></tr>
                    <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono text-gray-300">wallet_signature</td><td class="pr-4">string</td><td class="pr-4 text-gray-600">If wallet</td><td>Ed25519 signature (base58)</td></tr>
                    <tr><td class="py-2 pr-4 font-mono text-gray-300">org_domain</td><td class="pr-4">string</td><td class="pr-4 text-gray-600">No*</td><td>Domain of a verified organization to link</td></tr>
                  </tbody>
                </table>
              </div>
              <p class="text-xs text-gray-600 mt-2">* At least one identity type required. You can combine email, wallet, and org in a single request.</p>
            </div>

            <div>
              <h4 class="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Examples</h4>
              <div class="space-y-3">
                <div>
                  <p class="text-xs text-gray-500 mb-1">Send email verification code:</p>
                  <div class="relative">
                    <pre class="bg-gray-950 rounded border border-gray-800 px-4 py-3 text-xs font-mono text-gray-400 overflow-x-auto">curl -X POST {{ baseUrl }}/api/v1/verify-identity \
  -H "Content-Type: application/json" \
  -H "X-API-Key: YOUR_API_KEY" \
  -d '{"email": "you@example.com"}'</pre>
                    <button @click="copyToClipboard(curls.vi1, 'vi1')" class="absolute top-2 right-2 text-gray-600 hover:text-gray-400 cursor-pointer text-xs">
                      {{ copiedField === 'vi1' ? 'Copied!' : 'Copy' }}
                    </button>
                  </div>
                </div>
                <div>
                  <p class="text-xs text-gray-500 mb-1">Confirm email code:</p>
                  <div class="relative">
                    <pre class="bg-gray-950 rounded border border-gray-800 px-4 py-3 text-xs font-mono text-gray-400 overflow-x-auto">curl -X POST {{ baseUrl }}/api/v1/verify-identity \
  -H "Content-Type: application/json" \
  -H "X-API-Key: YOUR_API_KEY" \
  -d '{"email": "you@example.com", "code": "123456"}'</pre>
                    <button @click="copyToClipboard(curls.vi2, 'vi2')" class="absolute top-2 right-2 text-gray-600 hover:text-gray-400 cursor-pointer text-xs">
                      {{ copiedField === 'vi2' ? 'Copied!' : 'Copy' }}
                    </button>
                  </div>
                </div>
                <div>
                  <p class="text-xs text-gray-500 mb-1">Verify wallet:</p>
                  <div class="relative">
                    <pre class="bg-gray-950 rounded border border-gray-800 px-4 py-3 text-xs font-mono text-gray-400 overflow-x-auto">curl -X POST {{ baseUrl }}/api/v1/verify-identity \
  -H "Content-Type: application/json" \
  -H "X-API-Key: YOUR_API_KEY" \
  -d '{"wallet_pubkey": "BASE58_PUBKEY",
       "wallet_message": "R3L-verify:my-app",
       "wallet_signature": "BASE58_SIG"}'</pre>
                    <button @click="copyToClipboard(curls.vi3, 'vi3')" class="absolute top-2 right-2 text-gray-600 hover:text-gray-400 cursor-pointer text-xs">
                      {{ copiedField === 'vi3' ? 'Copied!' : 'Copy' }}
                    </button>
                  </div>
                </div>
                <div>
                  <p class="text-xs text-gray-500 mb-1">Link to organization (requires verified email with matching domain):</p>
                  <div class="relative">
                    <pre class="bg-gray-950 rounded border border-gray-800 px-4 py-3 text-xs font-mono text-gray-400 overflow-x-auto">curl -X POST {{ baseUrl }}/api/v1/verify-identity \
  -H "Content-Type: application/json" \
  -H "X-API-Key: YOUR_API_KEY" \
  -d '{"org_domain": "example.com"}'</pre>
                    <button @click="copyToClipboard(curls.vi4, 'vi4')" class="absolute top-2 right-2 text-gray-600 hover:text-gray-400 cursor-pointer text-xs">
                      {{ copiedField === 'vi4' ? 'Copied!' : 'Copy' }}
                    </button>
                  </div>
                </div>
                <div>
                  <p class="text-xs text-gray-500 mb-1">All types at once (email + wallet + org):</p>
                  <div class="relative">
                    <pre class="bg-gray-950 rounded border border-gray-800 px-4 py-3 text-xs font-mono text-gray-400 overflow-x-auto">curl -X POST {{ baseUrl }}/api/v1/verify-identity \
  -H "Content-Type: application/json" \
  -H "X-API-Key: YOUR_API_KEY" \
  -d '{"email": "you@example.com",
       "wallet_pubkey": "BASE58_PUBKEY",
       "wallet_message": "R3L-verify:my-app",
       "wallet_signature": "BASE58_SIG",
       "org_domain": "example.com"}'</pre>
                    <button @click="copyToClipboard(curls.vi5, 'vi5')" class="absolute top-2 right-2 text-gray-600 hover:text-gray-400 cursor-pointer text-xs">
                      {{ copiedField === 'vi5' ? 'Copied!' : 'Copy' }}
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

      </section>

      <!-- ============ ATTEST CONTENT ============ -->
      <section v-show="activeSection === 'content'" class="space-y-6">
        <h2 class="text-lg font-semibold border-b border-gray-800 pb-2">Attest Content</h2>

        <p class="text-sm text-gray-400">Verify and attest any content — files, URLs, or text — through a single endpoint. Use <span class="font-mono text-gray-300">/attest-content</span> for one at a time, or <span class="font-mono text-gray-300">/attest-content/batch</span> for multiple. All requests use <span class="font-mono text-gray-300">multipart/form-data</span>.</p>

        <!-- POST /api/v1/attest-content -->
        <div class="bg-gray-900 rounded-lg border border-blue-800/40 overflow-hidden">
          <div class="flex items-center gap-3 px-5 py-3 border-b border-gray-800">
            <span class="bg-blue-900/60 text-blue-400 text-xs font-mono px-2 py-0.5 rounded">POST</span>
            <span class="font-mono text-sm text-gray-200">/api/v1/attest-content</span>
            <span class="text-xs text-yellow-500">auth optional</span>
          </div>
          <div class="px-5 py-4 space-y-4">
            <p class="text-sm text-gray-400">Verify and attest <strong>one</strong> content item. Provide exactly one of: file, url, or text. Files are verified for C2PA metadata. All content is hashed, stored, and attested on Solana.</p>

            <div>
              <h4 class="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Request Body <span class="text-gray-600 normal-case">(multipart/form-data)</span></h4>
              <div class="overflow-x-auto">
                <table class="w-full text-xs">
                  <thead>
                    <tr class="text-left text-gray-500 border-b border-gray-800">
                      <th class="pb-2 pr-4">Field</th>
                      <th class="pb-2 pr-4">Type</th>
                      <th class="pb-2 pr-4">Required</th>
                      <th class="pb-2">Description</th>
                    </tr>
                  </thead>
                  <tbody class="text-gray-400">
                    <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono text-gray-300">file</td><td class="pr-4">file</td><td class="pr-4 text-gray-600">No*</td><td>Media file (max 50MB). Supports image/*, video/*, audio/*, PDF, text/*</td></tr>
                    <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono text-gray-300">url</td><td class="pr-4">string</td><td class="pr-4 text-gray-600">No*</td><td>URL to fetch and attest</td></tr>
                    <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono text-gray-300">text</td><td class="pr-4">string</td><td class="pr-4 text-gray-600">No*</td><td>Plaintext content to attest</td></tr>
                    <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono text-gray-300">title</td><td class="pr-4">string</td><td class="pr-4 text-gray-600">No</td><td>Optional title (for text content)</td></tr>
                    <tr class="border-b border-gray-800/50"><td class="py-2 pr-4 font-mono text-gray-300">store_content</td><td class="pr-4">string</td><td class="pr-4 text-gray-600">No</td><td>Store content on server. Default: <span class="font-mono">"true"</span></td></tr>
                    <tr><td class="py-2 pr-4 font-mono text-gray-300">private_mode</td><td class="pr-4">string</td><td class="pr-4 text-gray-600">No</td><td>DB only, skip Solana. Default: <span class="font-mono">"false"</span></td></tr>
                  </tbody>
                </table>
              </div>
              <p class="text-xs text-gray-600 mt-2">* Exactly one of <span class="font-mono">file</span>, <span class="font-mono">url</span>, or <span class="font-mono">text</span> per request.</p>
            </div>

            <div>
              <h4 class="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Response</h4>
              <pre class="bg-gray-950 rounded border border-gray-800 px-4 py-3 text-xs font-mono text-gray-400 overflow-x-auto">{
  "type": "file",
  "signature": "5abc...tx_sig",
  "attestation_pda": "Abc123...pda",
  "content_hash": "sha256hex...",
  "verify_output": { "has_c2pa": true, "issuer": "...", ... },
  "existing": false,
  "private": false
}</pre>
            </div>

            <div>
              <h4 class="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Examples</h4>
              <div class="space-y-3">
                <div>
                  <p class="text-xs text-gray-500 mb-1">Attest a file:</p>
                  <div class="relative">
                    <pre class="bg-gray-950 rounded border border-gray-800 px-4 py-3 text-xs font-mono text-gray-400 overflow-x-auto">curl -X POST {{ baseUrl }}/api/v1/attest-content \
  -H "X-API-Key: YOUR_API_KEY" \
  -F "file=@photo.jpg"</pre>
                    <button @click="copyToClipboard(curls.cf1, 'cf1')" class="absolute top-2 right-2 text-gray-600 hover:text-gray-400 cursor-pointer text-xs">
                      {{ copiedField === 'cf1' ? 'Copied!' : 'Copy' }}
                    </button>
                  </div>
                </div>
                <div>
                  <p class="text-xs text-gray-500 mb-1">Attest a URL:</p>
                  <div class="relative">
                    <pre class="bg-gray-950 rounded border border-gray-800 px-4 py-3 text-xs font-mono text-gray-400 overflow-x-auto">curl -X POST {{ baseUrl }}/api/v1/attest-content \
  -H "X-API-Key: YOUR_API_KEY" \
  -F "url=https://example.com/article"</pre>
                    <button @click="copyToClipboard(curls.cu1, 'cu1')" class="absolute top-2 right-2 text-gray-600 hover:text-gray-400 cursor-pointer text-xs">
                      {{ copiedField === 'cu1' ? 'Copied!' : 'Copy' }}
                    </button>
                  </div>
                </div>
                <div>
                  <p class="text-xs text-gray-500 mb-1">Attest text:</p>
                  <div class="relative">
                    <pre class="bg-gray-950 rounded border border-gray-800 px-4 py-3 text-xs font-mono text-gray-400 overflow-x-auto">curl -X POST {{ baseUrl }}/api/v1/attest-content \
  -H "X-API-Key: YOUR_API_KEY" \
  -F "text=Hello, world!" \
  -F "title=Test"</pre>
                    <button @click="copyToClipboard(curls.ct1, 'ct1')" class="absolute top-2 right-2 text-gray-600 hover:text-gray-400 cursor-pointer text-xs">
                      {{ copiedField === 'ct1' ? 'Copied!' : 'Copy' }}
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- POST /api/v1/attest-content/batch -->
        <div class="bg-gray-900 rounded-lg border border-blue-800/40 overflow-hidden">
          <div class="flex items-center gap-3 px-5 py-3 border-b border-gray-800">
            <span class="bg-blue-900/60 text-blue-400 text-xs font-mono px-2 py-0.5 rounded">POST</span>
            <span class="font-mono text-sm text-gray-200">/api/v1/attest-content/batch</span>
            <span class="text-xs text-yellow-500">auth optional</span>
          </div>
          <div class="px-5 py-4 space-y-4">
            <p class="text-sm text-gray-400">Verify and attest <strong>multiple</strong> content items in a single call. Include any combination of file, url, and text — each creates a separate attestation.</p>

            <div>
              <h4 class="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Example</h4>
              <div class="relative">
                <pre class="bg-gray-950 rounded border border-gray-800 px-4 py-3 text-xs font-mono text-gray-400 overflow-x-auto">curl -X POST {{ baseUrl }}/api/v1/attest-content/batch \
  -H "X-API-Key: YOUR_API_KEY" \
  -F "file=@photo.jpg" \
  -F "url=https://example.com/article" \
  -F "text=Hello, world!"</pre>
                <button @click="copyToClipboard(curls.cb1, 'cb1')" class="absolute top-2 right-2 text-gray-600 hover:text-gray-400 cursor-pointer text-xs">
                  {{ copiedField === 'cb1' ? 'Copied!' : 'Copy' }}
                </button>
              </div>
              <p class="text-xs text-gray-600 mt-1">Returns <span class="font-mono text-gray-500">{"results": [...]}</span> with one attestation result per content item.</p>
            </div>
          </div>
        </div>
      </section>

      <!-- ============ LOOKUP ============ -->
      <section v-show="activeSection === 'lookup'" class="space-y-6">
        <h2 class="text-lg font-semibold border-b border-gray-800 pb-2">Lookup Attestation</h2>

        <div class="bg-gray-900 rounded-lg border border-gray-800 overflow-hidden">
          <div class="flex items-center gap-3 px-5 py-3 border-b border-gray-800">
            <span class="bg-green-900/60 text-green-400 text-xs font-mono px-2 py-0.5 rounded">GET</span>
            <span class="font-mono text-sm text-gray-200">/api/attestation/{content_hash}</span>
          </div>
          <div class="px-5 py-4 space-y-4">
            <p class="text-sm text-gray-400">Look up a single attestation by SHA-256 content hash. Checks the database first, then falls back to on-chain PDA deserialization.</p>

            <div>
              <h4 class="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Path Parameters</h4>
              <div class="overflow-x-auto">
                <table class="w-full text-xs">
                  <thead>
                    <tr class="text-left text-gray-500 border-b border-gray-800">
                      <th class="pb-2 pr-4">Param</th>
                      <th class="pb-2 pr-4">Type</th>
                      <th class="pb-2">Description</th>
                    </tr>
                  </thead>
                  <tbody class="text-gray-400">
                    <tr><td class="py-2 pr-4 font-mono text-gray-300">content_hash</td><td class="pr-4">string</td><td>SHA-256 hash (64-char hex)</td></tr>
                  </tbody>
                </table>
              </div>
            </div>

            <div>
              <h4 class="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Response</h4>
              <pre class="bg-gray-950 rounded border border-gray-800 px-4 py-3 text-xs font-mono text-gray-400 overflow-x-auto">{
  "content_hash": "abc123...",
  "has_c2pa": true,
  "trust_list_match": "official",
  "validation_state": "valid",
  "issuer": "Adobe Inc",
  "signing_time": "2025-01-15T...",
  "timestamp": 1705000000,
  "proof_type": "trusted_verifier",
  "content_type": "file",
  "stored": true
}</pre>
            </div>

            <div>
              <h4 class="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Example</h4>
              <div class="relative">
                <pre class="bg-gray-950 rounded border border-gray-800 px-4 py-3 text-xs font-mono text-gray-400 overflow-x-auto">curl {{ baseUrl }}/api/attestation/CONTENT_HASH</pre>
                <button @click="copyToClipboard(curls.lk1, 'lk1')" class="absolute top-2 right-2 text-gray-600 hover:text-gray-400 cursor-pointer text-xs">
                  {{ copiedField === 'lk1' ? 'Copied!' : 'Copy' }}
                </button>
              </div>
            </div>
          </div>
        </div>
      </section>

      <!-- ============ LIST ============ -->
      <section v-show="activeSection === 'list'" class="space-y-6">
        <h2 class="text-lg font-semibold border-b border-gray-800 pb-2">List Attestations</h2>

        <div class="bg-gray-900 rounded-lg border border-gray-800 overflow-hidden">
          <div class="flex items-center gap-3 px-5 py-3 border-b border-gray-800">
            <span class="bg-green-900/60 text-green-400 text-xs font-mono px-2 py-0.5 rounded">GET</span>
            <span class="font-mono text-sm text-gray-200">/api/attestations</span>
          </div>
          <div class="px-5 py-4 space-y-4">
            <p class="text-sm text-gray-400">List all public attestations, ordered by timestamp descending.</p>

            <div>
              <h4 class="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Response</h4>
              <pre class="bg-gray-950 rounded border border-gray-800 px-4 py-3 text-xs font-mono text-gray-400 overflow-x-auto">[
  {
    "content_hash": "abc123...",
    "proof_type": "trusted_verifier",
    "timestamp": 1705000000,
    "issuer": "Adobe Inc",
    "content_type": "file",
    "stored": true
  },
  ...
]</pre>
            </div>
          </div>
        </div>
      </section>

      <!-- ============ QUERY VERDICT ============ -->
      <section v-show="activeSection === 'query'" class="space-y-6">
        <h2 class="text-lg font-semibold border-b border-gray-800 pb-2">Query Verdict</h2>

        <!-- Single query -->
        <div class="bg-gray-900 rounded-lg border border-green-800/40 overflow-hidden">
          <div class="flex items-center gap-3 px-5 py-3 border-b border-gray-800">
            <span class="bg-green-900/60 text-green-400 text-xs font-mono px-2 py-0.5 rounded">GET</span>
            <span class="font-mono text-sm text-gray-200">/api/v1/query/{content_hash}</span>
            <span class="text-xs text-green-500">Truth Oracle</span>
          </div>
          <div class="px-5 py-4 space-y-4">
            <p class="text-sm text-gray-400">Get a structured trust verdict for a content hash. Designed for external consumers and AI agents.</p>

            <div>
              <h4 class="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Verdict Values</h4>
              <div class="flex gap-3 text-xs">
                <span class="bg-green-900/40 text-green-400 px-2 py-1 rounded font-mono">trusted</span>
                <span class="bg-blue-900/40 text-blue-400 px-2 py-1 rounded font-mono">attested</span>
                <span class="bg-gray-800 text-gray-500 px-2 py-1 rounded font-mono">unknown</span>
              </div>
              <p class="text-xs text-gray-600 mt-2"><span class="text-green-400 font-mono">trusted</span> = C2PA signed by official/curated signer. <span class="text-blue-400 font-mono">attested</span> = on-chain but unknown signer. <span class="text-gray-500 font-mono">unknown</span> = no attestation found.</p>
            </div>

            <div>
              <h4 class="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Example</h4>
              <div class="relative">
                <pre class="bg-gray-950 rounded border border-gray-800 px-4 py-3 text-xs font-mono text-gray-400 overflow-x-auto">curl {{ baseUrl }}/api/v1/query/CONTENT_HASH</pre>
                <button @click="copyToClipboard(curls.qv1, 'qv1')" class="absolute top-2 right-2 text-gray-600 hover:text-gray-400 cursor-pointer text-xs">
                  {{ copiedField === 'qv1' ? 'Copied!' : 'Copy' }}
                </button>
              </div>
            </div>
          </div>
        </div>

        <!-- Batch query -->
        <div class="bg-gray-900 rounded-lg border border-green-800/40 overflow-hidden">
          <div class="flex items-center gap-3 px-5 py-3 border-b border-gray-800">
            <span class="bg-blue-900/60 text-blue-400 text-xs font-mono px-2 py-0.5 rounded">POST</span>
            <span class="font-mono text-sm text-gray-200">/api/v1/query/batch</span>
          </div>
          <div class="px-5 py-4 space-y-4">
            <p class="text-sm text-gray-400">Batch query up to 50 content hashes at once. Body is a JSON array of hex hash strings.</p>

            <div>
              <h4 class="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Example</h4>
              <div class="relative">
                <pre class="bg-gray-950 rounded border border-gray-800 px-4 py-3 text-xs font-mono text-gray-400 overflow-x-auto">curl -X POST {{ baseUrl }}/api/v1/query/batch \
  -H "Content-Type: application/json" \
  -d '["hash1...", "hash2..."]'</pre>
                <button @click="copyToClipboard(curls.qb1, 'qb1')" class="absolute top-2 right-2 text-gray-600 hover:text-gray-400 cursor-pointer text-xs">
                  {{ copiedField === 'qb1' ? 'Copied!' : 'Copy' }}
                </button>
              </div>
            </div>
          </div>
        </div>
      </section>

      <!-- ============ SIMILARITY SEARCH ============ -->
      <section v-show="activeSection === 'similar'" class="space-y-6">
        <h2 class="text-lg font-semibold border-b border-gray-800 pb-2">Similarity Search</h2>

        <!-- By file -->
        <div class="bg-gray-900 rounded-lg border border-orange-800/40 overflow-hidden">
          <div class="flex items-center gap-3 px-5 py-3 border-b border-gray-800">
            <span class="bg-blue-900/60 text-blue-400 text-xs font-mono px-2 py-0.5 rounded">POST</span>
            <span class="font-mono text-sm text-gray-200">/api/v1/similar</span>
          </div>
          <div class="px-5 py-4 space-y-4">
            <p class="text-sm text-gray-400">Upload a file and find similar attested content using TLSH (byte-level) and CLIP (semantic) similarity.</p>

            <div>
              <h4 class="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Request Body <span class="text-gray-600 normal-case">(multipart/form-data)</span></h4>
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
                    <tr><td class="py-2 pr-4 font-mono text-gray-300">file</td><td class="pr-4">file</td><td>File to search for (max 50MB)</td></tr>
                  </tbody>
                </table>
              </div>
            </div>

            <div>
              <h4 class="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Match Types</h4>
              <div class="flex flex-wrap gap-2 text-xs">
                <span class="bg-green-900/40 text-green-400 px-2 py-1 rounded font-mono">exact</span>
                <span class="bg-blue-900/40 text-blue-400 px-2 py-1 rounded font-mono">near_duplicate</span>
                <span class="bg-yellow-900/40 text-yellow-400 px-2 py-1 rounded font-mono">visual_match</span>
                <span class="bg-gray-800 text-gray-500 px-2 py-1 rounded font-mono">unrelated</span>
              </div>
            </div>

            <div>
              <h4 class="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Example</h4>
              <div class="relative">
                <pre class="bg-gray-950 rounded border border-gray-800 px-4 py-3 text-xs font-mono text-gray-400 overflow-x-auto">curl -X POST {{ baseUrl }}/api/v1/similar \
  -F "file=@photo.jpg"</pre>
                <button @click="copyToClipboard(curls.sim1, 'sim1')" class="absolute top-2 right-2 text-gray-600 hover:text-gray-400 cursor-pointer text-xs">
                  {{ copiedField === 'sim1' ? 'Copied!' : 'Copy' }}
                </button>
              </div>
            </div>
          </div>
        </div>

        <!-- By hash -->
        <div class="bg-gray-900 rounded-lg border border-orange-800/40 overflow-hidden">
          <div class="flex items-center gap-3 px-5 py-3 border-b border-gray-800">
            <span class="bg-green-900/60 text-green-400 text-xs font-mono px-2 py-0.5 rounded">GET</span>
            <span class="font-mono text-sm text-gray-200">/api/v1/similar/{content_hash}</span>
          </div>
          <div class="px-5 py-4 space-y-4">
            <p class="text-sm text-gray-400">Find content similar to an existing attestation by its SHA-256 hash. Uses the stored TLSH + CLIP embeddings from the original attestation.</p>

            <div>
              <h4 class="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Example</h4>
              <div class="relative">
                <pre class="bg-gray-950 rounded border border-gray-800 px-4 py-3 text-xs font-mono text-gray-400 overflow-x-auto">curl {{ baseUrl }}/api/v1/similar/CONTENT_HASH</pre>
                <button @click="copyToClipboard(curls.sim2, 'sim2')" class="absolute top-2 right-2 text-gray-600 hover:text-gray-400 cursor-pointer text-xs">
                  {{ copiedField === 'sim2' ? 'Copied!' : 'Copy' }}
                </button>
              </div>
            </div>
          </div>
        </div>
      </section>

      <!-- ============ ACCOUNT ============ -->
      <section v-show="activeSection === 'account'" class="space-y-6">
        <h2 class="text-lg font-semibold border-b border-gray-800 pb-2">Account</h2>

        <!-- GET /api/v1/me -->
        <div class="bg-gray-900 rounded-lg border border-gray-800 overflow-hidden">
          <div class="flex items-center gap-3 px-5 py-3 border-b border-gray-800">
            <span class="bg-green-900/60 text-green-400 text-xs font-mono px-2 py-0.5 rounded">GET</span>
            <span class="font-mono text-sm text-gray-200">/api/v1/me</span>
            <span class="text-xs text-green-500">auth required</span>
          </div>
          <div class="px-5 py-4 space-y-4">
            <p class="text-sm text-gray-400">Get your account information.</p>

            <div>
              <h4 class="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Response (individual)</h4>
              <pre class="bg-gray-950 rounded border border-gray-800 px-4 py-3 text-xs font-mono text-gray-400 overflow-x-auto">{
  "type": "individual",
  "name": "My App",
  "email": "dev@example.com",
  "wallet_pubkey": "base58...",
  "auth_method": "email",
  "privacy_mode": false
}</pre>
            </div>

            <div>
              <h4 class="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Example</h4>
              <div class="relative">
                <pre class="bg-gray-950 rounded border border-gray-800 px-4 py-3 text-xs font-mono text-gray-400 overflow-x-auto">curl {{ baseUrl }}/api/v1/me \
  -H "X-API-Key: YOUR_API_KEY"</pre>
                <button @click="copyToClipboard(curls.me1, 'me1')" class="absolute top-2 right-2 text-gray-600 hover:text-gray-400 cursor-pointer text-xs">
                  {{ copiedField === 'me1' ? 'Copied!' : 'Copy' }}
                </button>
              </div>
            </div>
          </div>
        </div>

        <!-- PATCH /api/auth/me/privacy -->
        <div class="bg-gray-900 rounded-lg border border-gray-800 overflow-hidden">
          <div class="flex items-center gap-3 px-5 py-3 border-b border-gray-800">
            <span class="bg-yellow-900/60 text-yellow-400 text-xs font-mono px-2 py-0.5 rounded">PATCH</span>
            <span class="font-mono text-sm text-gray-200">/api/auth/me/privacy</span>
            <span class="text-xs text-green-500">auth required</span>
          </div>
          <div class="px-5 py-4 space-y-4">
            <p class="text-sm text-gray-400">Toggle privacy mode. When enabled, identity fields are kept off the Solana ledger (stored in database only).</p>

            <div>
              <h4 class="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Request Body <span class="text-gray-600 normal-case">(JSON)</span></h4>
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
                    <tr><td class="py-2 pr-4 font-mono text-gray-300">privacy_mode</td><td class="pr-4">boolean</td><td>Enable or disable privacy mode</td></tr>
                  </tbody>
                </table>
              </div>
            </div>

            <div>
              <h4 class="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-2">Example</h4>
              <div class="relative">
                <pre class="bg-gray-950 rounded border border-gray-800 px-4 py-3 text-xs font-mono text-gray-400 overflow-x-auto">curl -X PATCH {{ baseUrl }}/api/auth/me/privacy \
  -H "Content-Type: application/json" \
  -H "X-API-Key: YOUR_API_KEY" \
  -d '{"privacy_mode": true}'</pre>
                <button @click="copyToClipboard(curls.priv1, 'priv1')" class="absolute top-2 right-2 text-gray-600 hover:text-gray-400 cursor-pointer text-xs">
                  {{ copiedField === 'priv1' ? 'Copied!' : 'Copy' }}
                </button>
              </div>
            </div>
          </div>
        </div>
      </section>

    </div>
  </div>
</template>
