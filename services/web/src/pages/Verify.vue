<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import type { VerifyOutput, AttestResponse, MeResponse } from '../types'
import FileUpload from '../components/FileUpload.vue'
import { attestFile, attestUrl, attestText, getMe } from '../api'

type TabMode = 'file' | 'url' | 'text'
const activeTab = ref<TabMode>('file')

const result = ref<VerifyOutput | null>(null)
const file = ref<File | null>(null)

// URL mode
const urlInput = ref('')
const showUrlHeaders = ref(false)
const urlHeadersRaw = ref('')  // "Key: Value" per line

// Text mode
const textInput = ref('')
const textTitle = ref('')

// ── Logged-in identity ──
const loggedInUser = ref<MeResponse | null>(null)

async function loadIdentity() {
  const key = localStorage.getItem('r3l_api_key')
  if (!key) return
  try {
    loggedInUser.value = await getMe(key)
  } catch {
    loggedInUser.value = null
  }
}

const identityLabel = computed(() => {
  const u = loggedInUser.value
  if (!u) return null
  if (u.type === 'org') return u.org?.name || u.org?.domain || 'Organization'
  if (u.email) return u.email
  if (u.wallet_pubkey) return u.wallet_pubkey.slice(0, 4) + '...' + u.wallet_pubkey.slice(-4)
  return u.name || null
})

function onVerified(r: VerifyOutput, f: File) {
  result.value = r
  file.value = f
}

// --- Signer status ---
type SignerStatus = 'trusted_official' | 'trusted_curated' | 'unknown_signer' | 'revoked' | 'expired' | 'self_signed' | 'no_c2pa'

const signerStatus = computed<SignerStatus>(() => {
  if (!result.value || !result.value.has_c2pa) return 'no_c2pa'
  const codes = result.value.validation_codes || []
  if (codes.includes('signingCredential.revoked')) return 'revoked'
  if (codes.includes('signingCredential.expired')) return 'expired'
  if (codes.includes('signingCredential.untrusted')) {
    if (result.value.issuer && result.value.common_name
      && result.value.issuer === result.value.common_name) return 'self_signed'
    return 'unknown_signer'
  }
  if (result.value.trust_list_match === 'official') return 'trusted_official'
  if (result.value.trust_list_match === 'curated') return 'trusted_curated'
  return 'unknown_signer'
})

const signerConfig = computed(() => {
  const configs: Record<SignerStatus, { label: string; color: string; borderColor: string; desc: string }> = {
    trusted_official: { label: 'Trusted (C2PA Official)', color: 'text-green-400', borderColor: 'border-green-800', desc: 'Signed by a certificate on the C2PA official trust list.' },
    trusted_curated: { label: 'Trusted (r3l Curated)', color: 'text-green-400', borderColor: 'border-green-800', desc: 'Signed by a certificate on the r3l curated trust list.' },
    unknown_signer: { label: 'Unknown Signer', color: 'text-yellow-400', borderColor: 'border-yellow-800', desc: 'C2PA metadata present but the signing certificate is not recognized.' },
    revoked: { label: 'Revoked Certificate', color: 'text-red-400', borderColor: 'border-red-800', desc: 'The signing certificate has been revoked.' },
    expired: { label: 'Expired Certificate', color: 'text-orange-400', borderColor: 'border-orange-800', desc: 'The signing certificate has expired.' },
    self_signed: { label: 'Self-Signed', color: 'text-red-400', borderColor: 'border-red-800', desc: 'Signed with a self-signed certificate. Cannot verify origin.' },
    no_c2pa: { label: 'No C2PA', color: 'text-gray-400', borderColor: 'border-gray-700', desc: 'No embedded C2PA metadata found in this file.' },
  }
  return configs[signerStatus.value]
})

// --- Source type ---
const sourceTypeMap: Record<string, string> = {
  'http://cv.iptc.org/newscodes/digitalsourcetype/digitalCapture': 'Digital Capture (Camera/Device)',
  'https://cv.iptc.org/newscodes/digitalsourcetype/digitalCapture': 'Digital Capture (Camera/Device)',
  'http://cv.iptc.org/newscodes/digitalsourcetype/trainedAlgorithmicMedia': 'AI-Generated',
  'https://cv.iptc.org/newscodes/digitalsourcetype/trainedAlgorithmicMedia': 'AI-Generated',
  'http://cv.iptc.org/newscodes/digitalsourcetype/compositeWithTrainedAlgorithmicMedia': 'Composite (includes AI)',
  'https://cv.iptc.org/newscodes/digitalsourcetype/compositeWithTrainedAlgorithmicMedia': 'Composite (includes AI)',
  'http://cv.iptc.org/newscodes/digitalsourcetype/algorithmicMedia': 'Algorithmically Generated',
  'https://cv.iptc.org/newscodes/digitalsourcetype/algorithmicMedia': 'Algorithmically Generated',
  'http://cv.iptc.org/newscodes/digitalsourcetype/digitalArt': 'Digital Art',
  'https://cv.iptc.org/newscodes/digitalsourcetype/digitalArt': 'Digital Art',
  'http://cv.iptc.org/newscodes/digitalsourcetype/compositeCapture': 'Composite Capture',
  'https://cv.iptc.org/newscodes/digitalsourcetype/compositeCapture': 'Composite Capture',
}

const sourceTypeLabel = computed(() => {
  const raw = result.value?.digital_source_type
  if (!raw) return null
  return sourceTypeMap[raw] || raw
})

const sourceTypeColor = computed(() => {
  const raw = result.value?.digital_source_type || ''
  if (raw.includes('trainedAlgorithmicMedia') || raw.includes('algorithmicMedia')) return 'text-purple-400'
  if (raw.includes('digitalCapture')) return 'text-blue-400'
  return 'text-gray-100'
})

// --- Ingredients and actions ---
const ingredients = computed(() => {
  const raw = result.value?.ingredients
  if (!raw || !Array.isArray(raw)) return []
  return raw.map((ing: any) => ({
    title: ing.title || 'Unknown',
    format: ing.format || null,
    relationship: ing.relationship || null,
  }))
})

const actions = computed(() => {
  const raw = result.value?.actions
  if (!raw || !Array.isArray(raw)) return []
  return raw.map((a: any) => {
    const action = typeof a === 'string' ? a : (a.action || 'unknown')
    return action.replace(/^c2pa\./, '')
  })
})

// --- Copy to clipboard ---
const copiedField = ref<string | null>(null)

async function copyToClipboard(value: string, fieldLabel: string) {
  try {
    await navigator.clipboard.writeText(value)
    copiedField.value = fieldLabel
    setTimeout(() => { copiedField.value = null }, 1500)
  } catch { /* no-op */ }
}

// --- Attest ---
const attesting = ref(false)
const attestError = ref<string | null>(null)
const attestResult = ref<AttestResponse | null>(null)

const privateMode = ref(false)
const currentApiKey = computed(() => localStorage.getItem('r3l_api_key') || undefined)
const isPrivacyMode = computed(() => loggedInUser.value?.privacy_mode === true)

async function attestFileHandler() {
  if (!file.value) return
  attesting.value = true
  attestError.value = null
  try {
    attestResult.value = await attestFile(file.value, { apiKey: currentApiKey.value, privateMode: privateMode.value })
  } catch (e: any) {
    const raw = e.response?.data || e.message || 'Unknown error'
    attestError.value = typeof raw === 'string' ? raw : JSON.stringify(raw)
  } finally {
    attesting.value = false
  }
}

function parseUrlHeaders(): Record<string, string> | undefined {
  const raw = urlHeadersRaw.value.trim()
  if (!raw) return undefined
  const headers: Record<string, string> = {}
  for (const line of raw.split('\n')) {
    const idx = line.indexOf(':')
    if (idx > 0) {
      const key = line.slice(0, idx).trim()
      const val = line.slice(idx + 1).trim()
      if (key && val) headers[key] = val
    }
  }
  return Object.keys(headers).length > 0 ? headers : undefined
}

async function attestUrlHandler() {
  const url = urlInput.value.trim()
  if (!url) return
  attesting.value = true
  attestError.value = null
  try {
    attestResult.value = await attestUrl(url, true, currentApiKey.value, parseUrlHeaders(), privateMode.value)
  } catch (e: any) {
    const raw = e.response?.data || e.message || 'Unknown error'
    attestError.value = typeof raw === 'string' ? raw : JSON.stringify(raw)
  } finally {
    attesting.value = false
  }
}

async function attestTextHandler() {
  const text = textInput.value.trim()
  if (!text) return
  attesting.value = true
  attestError.value = null
  try {
    attestResult.value = await attestText(text, textTitle.value.trim() || undefined, true, currentApiKey.value, privateMode.value)
  } catch (e: any) {
    const raw = e.response?.data || e.message || 'Unknown error'
    attestError.value = typeof raw === 'string' ? raw : JSON.stringify(raw)
  } finally {
    attesting.value = false
  }
}

function reset() {
  result.value = null
  file.value = null
  urlInput.value = ''
  privateMode.value = false
  showUrlHeaders.value = false
  urlHeadersRaw.value = ''
  textInput.value = ''
  textTitle.value = ''
  attestResult.value = null
  attestError.value = null
  attesting.value = false
}

onMounted(loadIdentity)
</script>

<template>
  <div class="space-y-6">
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-bold">Attest Content</h1>
      <button v-if="result || attestResult" @click="reset" class="text-sm text-gray-400 hover:text-white transition-colors cursor-pointer">
        Start over
      </button>
    </div>

    <!-- Identity bar -->
    <div v-if="loggedInUser" class="bg-gray-900 rounded-lg border border-gray-800 px-4 py-2.5 flex items-center justify-between">
      <div class="flex items-center gap-2 text-sm">
        <span class="text-gray-500">Attesting as</span>
        <span class="text-gray-200 font-medium">{{ identityLabel }}</span>
        <span class="text-xs text-gray-600">({{ loggedInUser.type === 'org' ? 'org' : loggedInUser.auth_method }})</span>
        <span v-if="isPrivacyMode" class="text-xs bg-purple-900/50 text-purple-400 px-2 py-0.5 rounded-full border border-purple-800/50">Private</span>
      </div>
      <router-link to="/account" class="text-xs text-gray-500 hover:text-gray-300 transition-colors">Account</router-link>
    </div>
    <div v-else class="bg-gray-900/50 rounded-lg border border-gray-800/50 px-4 py-2.5 flex items-center justify-between">
      <span class="text-sm text-gray-600">Not logged in — attestations will be anonymous.</span>
      <router-link to="/account" class="text-xs text-gray-500 hover:text-gray-300 transition-colors">Sign in</router-link>
    </div>

    <!-- Tab bar -->
    <div v-if="!result && !attestResult" class="flex gap-1 bg-gray-900 rounded-lg p-1 border border-gray-800">
      <button
        v-for="tab in ([
          { key: 'file', label: 'File Upload' },
          { key: 'url', label: 'URL' },
          { key: 'text', label: 'Text' },
        ] as { key: TabMode; label: string }[])"
        :key="tab.key"
        @click="activeTab = tab.key"
        :class="[
          'flex-1 py-2 text-sm font-medium rounded-md transition-colors cursor-pointer',
          activeTab === tab.key ? 'bg-gray-800 text-white' : 'text-gray-500 hover:text-gray-300',
        ]"
      >{{ tab.label }}</button>
    </div>

    <!-- ═══ File Upload Tab ═══ -->
    <template v-if="activeTab === 'file' && !attestResult">
      <FileUpload v-if="!result" @verified="onVerified" />

      <template v-if="result">
        <div class="flex items-center gap-3">
          <span class="text-sm text-gray-500">{{ file?.name }}</span>
        </div>

        <!-- Content hash hero -->
        <div v-if="result.content_hash" class="bg-gray-900 rounded-lg border border-gray-800 p-4">
          <div class="flex items-center justify-between mb-2">
            <span class="text-xs text-gray-500 uppercase tracking-wider">Content Hash</span>
            <button @click="copyToClipboard(result.content_hash!, 'hash')" class="text-gray-500 hover:text-gray-300 cursor-pointer">
              <span v-if="copiedField === 'hash'" class="text-green-400 text-xs">Copied!</span>
              <svg v-else xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                <rect x="9" y="9" width="13" height="13" rx="2" ry="2" /><path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1" />
              </svg>
            </button>
          </div>
          <p class="text-sm font-mono text-gray-200 break-all">{{ result.content_hash }}</p>
        </div>

        <!-- Details table -->
        <div class="bg-gray-900 rounded-lg border border-gray-800 overflow-hidden divide-y divide-gray-800">
          <div v-if="result.format" class="flex items-center px-4 py-3">
            <span class="text-gray-400 w-44 shrink-0 text-sm">Media Type</span>
            <span class="text-sm text-gray-100">{{ result.format }}</span>
          </div>

          <!-- C2PA fields (only if present) -->
          <div v-if="result.has_c2pa" class="flex items-center px-4 py-3">
            <span class="text-gray-400 w-44 shrink-0 text-sm">C2PA Signer</span>
            <span :class="['text-sm font-medium', signerConfig.color]">{{ signerConfig.label }}</span>
          </div>
          <div v-if="sourceTypeLabel" class="flex px-4 py-3">
            <span class="text-gray-400 w-44 shrink-0 text-sm">Source Type</span>
            <span :class="['text-sm', sourceTypeColor]">{{ sourceTypeLabel }}</span>
          </div>
          <div v-if="result.issuer" class="flex items-center px-4 py-3">
            <span class="text-gray-400 w-44 shrink-0 text-sm">Issuer</span>
            <span class="text-sm text-gray-100">{{ result.issuer }}</span>
          </div>
          <div v-if="result.common_name" class="flex items-center px-4 py-3">
            <span class="text-gray-400 w-44 shrink-0 text-sm">Common Name</span>
            <span class="text-sm text-gray-100">{{ result.common_name }}</span>
          </div>
          <div v-if="result.software_agent" class="flex items-center px-4 py-3">
            <span class="text-gray-400 w-44 shrink-0 text-sm">Software Agent</span>
            <span class="text-sm text-gray-100">{{ result.software_agent }}</span>
          </div>
          <div v-if="result.signing_time" class="flex items-center px-4 py-3">
            <span class="text-gray-400 w-44 shrink-0 text-sm">Signing Time</span>
            <span class="text-sm text-gray-100">{{ result.signing_time }}</span>
          </div>
          <div v-if="result.validation_codes && result.validation_codes.length > 0" class="flex items-center px-4 py-3">
            <span class="text-gray-400 w-44 shrink-0 text-sm">Validation Codes</span>
            <div class="flex flex-wrap gap-1.5">
              <span v-for="code in result.validation_codes" :key="code" class="text-xs font-mono px-2 py-0.5 rounded bg-gray-800 text-gray-300">{{ code }}</span>
            </div>
          </div>
          <div v-if="ingredients.length > 0" class="px-4 py-3">
            <span class="text-gray-400 text-sm">Ingredients</span>
            <div class="mt-2 space-y-2">
              <div v-for="(ing, i) in ingredients" :key="i" class="flex items-center gap-3 bg-gray-800/50 rounded px-3 py-2">
                <span class="text-xs text-gray-500 shrink-0">{{ ing.relationship || 'ingredient' }}</span>
                <span class="text-sm text-gray-200">{{ ing.title }}</span>
                <span v-if="ing.format" class="text-xs text-gray-500">{{ ing.format }}</span>
              </div>
            </div>
          </div>
          <div v-if="actions.length > 0" class="px-4 py-3">
            <span class="text-gray-400 text-sm">Edit History</span>
            <div class="mt-2 flex flex-wrap gap-1.5">
              <span v-for="(action, i) in actions" :key="i" class="text-xs px-2 py-0.5 rounded bg-gray-800 text-gray-300">{{ action }}</span>
            </div>
          </div>
        </div>

        <!-- Private mode toggle + Attest button -->
        <div class="space-y-3">
          <div class="flex items-center justify-between bg-gray-900 rounded-lg border border-gray-800 px-4 py-2.5">
            <div>
              <span class="text-sm text-gray-300">Private attestation</span>
              <p class="text-xs text-gray-600">Store in database only — not on Solana or public search.</p>
            </div>
            <button @click="privateMode = !privateMode" :class="['relative inline-flex h-5 w-9 items-center rounded-full transition-colors cursor-pointer', privateMode ? 'bg-purple-600' : 'bg-gray-700']">
              <span :class="['inline-block h-3.5 w-3.5 rounded-full bg-white transition-transform', privateMode ? 'translate-x-4.5' : 'translate-x-0.5']" />
            </button>
          </div>
          <button
            @click="attestFileHandler"
            :disabled="attesting"
            :class="['w-full py-3 disabled:opacity-50 rounded-lg text-sm font-medium transition-colors cursor-pointer', privateMode ? 'bg-purple-600 hover:bg-purple-500' : 'bg-blue-600 hover:bg-blue-500']"
          >
            {{ attesting ? (privateMode ? 'Storing privately...' : 'Attesting on Solana...') : (privateMode ? 'Store Private Attestation' : 'Attest on Solana') }}
          </button>
          <p class="text-xs text-gray-600 text-center">
            {{ privateMode ? 'Stores a private record in the database. Not visible publicly.' : 'Stores a permanent on-chain record of this verification.' }}
          </p>
          <p v-if="attestError" class="text-red-400 text-sm text-center">{{ attestError }}</p>
        </div>
      </template>
    </template>

    <!-- ═══ URL Tab ═══ -->
    <template v-if="activeTab === 'url' && !attestResult">
      <div class="space-y-4">
        <p class="text-sm text-gray-400">
          Fetch a URL and create an on-chain attestation of its content at this point in time.
        </p>
        <div>
          <label class="block text-sm text-gray-400 mb-2">URL to attest</label>
          <input
            v-model="urlInput"
            type="url"
            placeholder="https://example.com/article..."
            class="w-full bg-gray-900 border border-gray-700 rounded-lg px-4 py-2.5 text-sm text-gray-200 placeholder-gray-600 focus:border-blue-500 focus:outline-none"
          />
        </div>
        <div>
          <button
            @click="showUrlHeaders = !showUrlHeaders"
            class="text-xs text-gray-500 hover:text-gray-300 transition-colors cursor-pointer flex items-center gap-1"
          >
            <span :class="showUrlHeaders ? 'rotate-90' : ''" class="inline-block transition-transform">&#9654;</span>
            Auth headers (optional)
          </button>
          <div v-if="showUrlHeaders" class="mt-2">
            <textarea
              v-model="urlHeadersRaw"
              rows="3"
              placeholder="Authorization: Bearer your-token&#10;Cookie: session=abc123"
              class="w-full bg-gray-900 border border-gray-700 rounded-lg px-4 py-2.5 text-sm text-gray-200 placeholder-gray-600 focus:border-blue-500 focus:outline-none resize-y font-mono"
            ></textarea>
            <p class="text-xs text-gray-600 mt-1">One header per line as Key: Value. Forwarded when fetching the URL.</p>
          </div>
        </div>
        <div class="flex items-center justify-between bg-gray-900 rounded-lg border border-gray-800 px-4 py-2.5">
          <div>
            <span class="text-sm text-gray-300">Private attestation</span>
            <p class="text-xs text-gray-600">Store in database only — not on Solana or public search.</p>
          </div>
          <button @click="privateMode = !privateMode" :class="['relative inline-flex h-5 w-9 items-center rounded-full transition-colors cursor-pointer', privateMode ? 'bg-purple-600' : 'bg-gray-700']">
            <span :class="['inline-block h-3.5 w-3.5 rounded-full bg-white transition-transform', privateMode ? 'translate-x-4.5' : 'translate-x-0.5']" />
          </button>
        </div>
        <button
          @click="attestUrlHandler"
          :disabled="attesting || !urlInput.trim()"
          :class="['w-full py-3 disabled:opacity-50 rounded-lg text-sm font-medium transition-colors cursor-pointer', privateMode ? 'bg-purple-600 hover:bg-purple-500' : 'bg-blue-600 hover:bg-blue-500']"
        >
          {{ attesting ? 'Fetching & Attesting...' : (privateMode ? 'Fetch & Store Privately' : 'Fetch & Attest on Solana') }}
        </button>
        <p class="text-xs text-gray-600 text-center">
          {{ privateMode ? 'Fetches the URL, hashes the content, and stores a private record.' : 'Fetches the URL, hashes the content, and stores a permanent on-chain attestation.' }}
        </p>
        <p v-if="attestError" class="text-red-400 text-sm text-center">{{ attestError }}</p>
      </div>
    </template>

    <!-- ═══ Text Tab ═══ -->
    <template v-if="activeTab === 'text' && !attestResult">
      <div class="space-y-4">
        <p class="text-sm text-gray-400">
          Attest any text content — CSV data, JSON, API responses, articles, or any plaintext.
        </p>
        <div>
          <label class="block text-sm text-gray-400 mb-2">Title (optional)</label>
          <input
            v-model="textTitle"
            type="text"
            placeholder="e.g. API response from example.com"
            class="w-full bg-gray-900 border border-gray-700 rounded-lg px-4 py-2.5 text-sm text-gray-200 placeholder-gray-600 focus:border-blue-500 focus:outline-none"
          />
        </div>
        <div>
          <label class="block text-sm text-gray-400 mb-2">Content</label>
          <textarea
            v-model="textInput"
            rows="8"
            placeholder="Paste text, CSV, JSON, or any content here..."
            class="w-full bg-gray-900 border border-gray-700 rounded-lg px-4 py-2.5 text-sm text-gray-200 placeholder-gray-600 focus:border-blue-500 focus:outline-none resize-y font-mono"
          ></textarea>
        </div>
        <div class="flex items-center justify-between bg-gray-900 rounded-lg border border-gray-800 px-4 py-2.5">
          <div>
            <span class="text-sm text-gray-300">Private attestation</span>
            <p class="text-xs text-gray-600">Store in database only — not on Solana or public search.</p>
          </div>
          <button @click="privateMode = !privateMode" :class="['relative inline-flex h-5 w-9 items-center rounded-full transition-colors cursor-pointer', privateMode ? 'bg-purple-600' : 'bg-gray-700']">
            <span :class="['inline-block h-3.5 w-3.5 rounded-full bg-white transition-transform', privateMode ? 'translate-x-4.5' : 'translate-x-0.5']" />
          </button>
        </div>
        <button
          @click="attestTextHandler"
          :disabled="attesting || !textInput.trim()"
          :class="['w-full py-3 disabled:opacity-50 rounded-lg text-sm font-medium transition-colors cursor-pointer', privateMode ? 'bg-purple-600 hover:bg-purple-500' : 'bg-blue-600 hover:bg-blue-500']"
        >
          {{ attesting ? (privateMode ? 'Storing privately...' : 'Attesting on Solana...') : (privateMode ? 'Store Private Attestation' : 'Attest on Solana') }}
        </button>
        <p class="text-xs text-gray-600 text-center">
          {{ privateMode ? 'Hashes the text content and stores a private record.' : 'Hashes the text content and stores a permanent on-chain attestation.' }}
        </p>
        <p v-if="attestError" class="text-red-400 text-sm text-center">{{ attestError }}</p>
      </div>
    </template>

    <!-- ═══ Results (shared across all tabs) ═══ -->
    <div v-if="attestResult" class="space-y-4">
      <h2 class="text-lg font-semibold">{{ attestResult.private ? 'Private Attestation' : 'On-chain Result' }}</h2>

      <div :class="['bg-gray-900 rounded-lg border p-4 space-y-2', attestResult.private ? 'border-purple-900' : 'border-green-900']">
        <div class="flex items-center gap-2">
          <span :class="attestResult.private ? 'text-purple-400' : 'text-green-400'">&#10003;</span>
          <span :class="['text-sm font-medium', attestResult.private ? 'text-purple-400' : 'text-green-400']">
            {{ attestResult.existing ? 'Attestation already exists' : attestResult.private ? 'Private attestation recorded (database only)' : 'Attestation recorded on Solana' }}
          </span>
        </div>
        <div class="space-y-1">
          <div v-if="!attestResult.private" class="flex items-center gap-2">
            <span class="text-xs text-gray-500 w-20 shrink-0">Tx</span>
            <span class="text-xs font-mono text-gray-300 truncate flex-1">{{ attestResult.signature || '(existing)' }}</span>
            <button v-if="attestResult.signature" @click="copyToClipboard(attestResult.signature, 'tx')" class="shrink-0 text-gray-500 hover:text-gray-300 cursor-pointer">
              <span v-if="copiedField === 'tx'" class="text-green-400 text-xs">Copied</span>
              <span v-else class="text-xs">Copy</span>
            </button>
          </div>
          <div v-if="attestResult.attestation_pda" class="flex items-center gap-2">
            <span class="text-xs text-gray-500 w-20 shrink-0">PDA</span>
            <span class="text-xs font-mono text-gray-300 truncate flex-1">{{ attestResult.attestation_pda }}</span>
            <button @click="copyToClipboard(attestResult.attestation_pda!, 'pda')" class="shrink-0 text-gray-500 hover:text-gray-300 cursor-pointer">
              <span v-if="copiedField === 'pda'" class="text-green-400 text-xs">Copied</span>
              <span v-else class="text-xs">Copy</span>
            </button>
          </div>
          <div class="flex items-center gap-2">
            <span class="text-xs text-gray-500 w-20 shrink-0">Hash</span>
            <span class="text-xs font-mono text-gray-300 truncate flex-1">{{ attestResult.content_hash }}</span>
            <button @click="copyToClipboard(attestResult.content_hash, 'content_hash')" class="shrink-0 text-gray-500 hover:text-gray-300 cursor-pointer">
              <span v-if="copiedField === 'content_hash'" class="text-green-400 text-xs">Copied</span>
              <span v-else class="text-xs">Copy</span>
            </button>
          </div>
        </div>
        <p v-if="attestResult.private" class="text-xs text-gray-600 mt-2">This attestation is stored privately and won't appear in public searches or on-chain.</p>
      </div>

      <router-link
        v-if="!attestResult.private"
        :to="'/search/' + attestResult.content_hash"
        class="inline-block text-sm text-blue-400 hover:text-blue-300 transition-colors"
      >
        View attestation &rarr;
      </router-link>
    </div>
  </div>
</template>
