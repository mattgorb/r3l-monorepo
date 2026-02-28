<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import type { AttestationResponse, AttestationListItem, SimilarResponse } from '../types'
import { lookupAttestation, listAttestations, queryVerdict, searchSimilarByFile, searchSimilarByHash, getContentUrl } from '../api'

const route = useRoute()
const router = useRouter()

// --- State ---
const hashInput = ref('')
const file = ref<File | null>(null)
const loading = ref(false)
const error = ref('')

// Attestation detail (from hash lookup)
const attestation = ref<AttestationResponse | null>(null)
const verdict = ref<any>(null)
const notFound = ref(false)

// Similarity results
const similarResult = ref<SimilarResponse | null>(null)

// All attestations list
const allAttestations = ref<AttestationListItem[]>([])
const listLoading = ref(false)

// --- Fetch all attestations ---
async function fetchAll() {
  listLoading.value = true
  try {
    allAttestations.value = await listAttestations()
  } catch { /* silent */ } finally {
    listLoading.value = false
  }
}

// --- Search by hash: exact lookup + similarity ---
async function searchByHash(h?: string) {
  const query = (h || hashInput.value).trim()
  if (!query) return
  hashInput.value = query
  loading.value = true
  error.value = ''
  attestation.value = null
  verdict.value = null
  notFound.value = false
  similarResult.value = null
  file.value = null

  try {
    // Run exact lookup + similarity in parallel
    const [att, vrd, sim] = await Promise.all([
      lookupAttestation(query).catch(() => null),
      queryVerdict(query).catch(() => null),
      searchSimilarByHash(query).catch(() => null),
    ])
    if (att) {
      attestation.value = att
      verdict.value = vrd
    } else {
      notFound.value = true
    }
    similarResult.value = sim
  } catch (e: any) {
    error.value = e.response?.data?.detail || e.message || 'Search failed'
  } finally {
    loading.value = false
  }

  // Update URL without triggering watch loop
  if (route.params.hash !== query) {
    router.replace(`/search/${query}`)
  }
}

// --- Search by file: similarity only ---
async function searchByFile() {
  if (!file.value) return
  loading.value = true
  error.value = ''
  attestation.value = null
  verdict.value = null
  notFound.value = false
  similarResult.value = null
  hashInput.value = ''

  try {
    similarResult.value = await searchSimilarByFile(file.value)
  } catch (e: any) {
    error.value = e.response?.data?.detail || e.message || 'Search failed'
  } finally {
    loading.value = false
  }

  if (route.params.hash) {
    router.replace('/search')
  }
}

function onDrop(e: DragEvent) {
  e.preventDefault()
  const f = e.dataTransfer?.files[0]
  if (f) { file.value = f; searchByFile() }
}

function onFileSelect(e: Event) {
  const input = e.target as HTMLInputElement
  const f = input.files?.[0]
  if (f) { file.value = f; searchByFile() }
}

// --- Navigate to detail for a match ---
function viewDetail(contentHash: string) {
  router.push(`/search/${contentHash}`)
}

function selectItem(item: AttestationListItem) {
  router.push(`/search/${item.content_hash}`)
}

function backToSearch() {
  attestation.value = null
  verdict.value = null
  notFound.value = false
  similarResult.value = null
  error.value = ''
  hashInput.value = ''
  file.value = null
  router.push('/search')
}

// --- Auto-search from URL param ---
onMounted(() => {
  fetchAll()
  const paramHash = route.params.hash as string
  if (paramHash) {
    searchByHash(paramHash)
  }
})

watch(() => route.params.hash, (newHash) => {
  if (newHash && typeof newHash === 'string') {
    searchByHash(newHash)
  }
})

// --- Helpers ---
function formatTime(ts: number): string {
  if (!ts) return ''
  return new Date(ts * 1000).toLocaleDateString('en-US', {
    month: 'short', day: 'numeric', year: 'numeric',
    hour: '2-digit', minute: '2-digit',
  })
}

function truncHash(h: string): string {
  return h && h.length > 16 ? h.slice(0, 8) + '...' + h.slice(-8) : h
}

// --- Verdict config ---
const verdictConfig = computed(() => {
  const v = verdict.value?.verdict
  const configs: Record<string, { label: string; color: string; bg: string; border: string }> = {
    trusted: { label: 'TRUSTED', color: 'text-green-400', bg: 'bg-green-950/80', border: 'border-green-600' },
    attested: { label: 'ATTESTED', color: 'text-blue-400', bg: 'bg-blue-950/80', border: 'border-blue-600' },
    unknown: { label: 'UNKNOWN', color: 'text-gray-400', bg: 'bg-gray-900', border: 'border-gray-700' },
  }
  return configs[v] || null
})

// --- Signer config ---
const signerConfig = computed(() => {
  if (!attestation.value) return null
  const match = attestation.value.trust_list_match
  const configs: Record<string, { label: string; color: string; bg: string; desc: string }> = {
    official: { label: 'Trusted (C2PA Official)', color: 'text-green-400', bg: 'bg-green-950 border-green-800', desc: 'Signed by a certificate on the C2PA official trust list.' },
    curated: { label: 'Trusted (r3l Curated)', color: 'text-green-400', bg: 'bg-green-950 border-green-800', desc: 'Signed by a certificate on the r3l curated trust list.' },
    untrusted: { label: 'Unknown Signer', color: 'text-yellow-400', bg: 'bg-yellow-950 border-yellow-800', desc: 'C2PA metadata present but the signing certificate is not recognized.' },
  }
  return configs[match] || {
    label: match || 'Unknown',
    color: 'text-gray-400',
    bg: 'bg-gray-900 border-gray-700',
    desc: '',
  }
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
  const raw = attestation.value?.digital_source_type
  if (!raw) return null
  return sourceTypeMap[raw] || raw
})

const sourceTypeColor = computed(() => {
  const raw = attestation.value?.digital_source_type || ''
  if (raw.includes('trainedAlgorithmicMedia') || raw.includes('algorithmicMedia')) return 'text-purple-400'
  if (raw.includes('digitalCapture')) return 'text-blue-400'
  return 'text-gray-300'
})

// --- Copy ---
const copiedField = ref<string | null>(null)

async function copyToClipboard(value: string, fieldLabel: string) {
  try {
    await navigator.clipboard.writeText(value)
    copiedField.value = fieldLabel
    setTimeout(() => { copiedField.value = null }, 1500)
  } catch { /* no-op */ }
}

// --- Match config for similarity ---
const matchConfig: Record<string, { label: string; color: string; border: string; bg: string }> = {
  exact: { label: 'Exact Match', color: 'text-green-400', border: 'border-green-700', bg: 'bg-green-950' },
  near_duplicate: { label: 'Near Duplicate', color: 'text-blue-400', border: 'border-blue-700', bg: 'bg-blue-950' },
  visual_match: { label: 'Visual Match', color: 'text-yellow-400', border: 'border-yellow-700', bg: 'bg-yellow-950' },
  unrelated: { label: 'Unrelated', color: 'text-gray-400', border: 'border-gray-700', bg: 'bg-gray-900' },
}

// --- Content type config ---
const defaultBadge = { label: 'File', color: 'text-gray-300', bg: 'bg-gray-800' }

function contentTypeBadge(ct?: string): { label: string; color: string; bg: string } {
  if (ct === 'url') return { label: 'URL', color: 'text-cyan-400', bg: 'bg-cyan-950' }
  if (ct === 'text') return { label: 'Text', color: 'text-amber-400', bg: 'bg-amber-950' }
  return defaultBadge
}

// --- Stored content preview check ---
function isPreviewableImage(att: any): boolean {
  return att?.stored && att?.mime_type?.startsWith('image/')
}

// --- Are we showing detail view? ---
const showingDetail = computed(() => !!attestation.value)
</script>

<template>
  <div class="space-y-6">
    <h1 class="text-2xl font-bold">Search</h1>

    <!-- Back button when viewing detail -->
    <button v-if="showingDetail || notFound || similarResult" @click="backToSearch" class="text-sm text-gray-400 hover:text-gray-200 cursor-pointer flex items-center gap-1 transition-colors">
      <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M15 19l-7-7 7-7" /></svg>
      Back to search
    </button>

    <!-- Search inputs (hidden when viewing detail) -->
    <div v-if="!showingDetail && !similarResult" class="space-y-6">
      <p class="text-gray-400 text-sm -mt-4">
        Look up attestations by hash or upload a file to find similar attested content.
      </p>

      <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
        <!-- File upload -->
        <div
          class="border border-dashed border-gray-700 rounded-lg p-8 text-center cursor-pointer hover:border-gray-500 transition-colors"
          @drop="onDrop"
          @dragover.prevent
          @click="($refs.fileInput as HTMLInputElement)?.click()"
        >
          <input ref="fileInput" type="file" class="hidden" accept="image/*,video/*,audio/*,application/pdf,text/plain,text/csv,text/markdown,text/html,.txt,.csv,.md,.html" @change="onFileSelect" />
          <div v-if="file" class="text-sm text-gray-300">
            <span class="font-medium text-white">{{ file.name }}</span>
            <span class="text-gray-500 ml-2">({{ (file.size / 1024).toFixed(0) }} KB)</span>
          </div>
          <div v-else>
            <p class="text-gray-400 text-sm">Drop a file here or click to upload</p>
            <p class="text-gray-600 text-xs mt-1">Find similar attested content by file</p>
          </div>
        </div>

        <!-- Hash input -->
        <div>
          <label class="block text-sm text-gray-400 mb-2">Search by content hash</label>
          <form @submit.prevent="() => searchByHash()" class="flex gap-2">
            <input
              v-model="hashInput"
              type="text"
              placeholder="SHA-256 content hash..."
              class="flex-1 bg-gray-900 border border-gray-700 rounded-lg px-4 py-2.5 text-sm font-mono text-gray-200 placeholder-gray-600 focus:border-blue-500 focus:outline-none"
            />
            <button
              type="submit"
              :disabled="!hashInput.trim() || loading"
              class="px-5 py-2.5 bg-blue-600 hover:bg-blue-500 disabled:opacity-50 rounded-lg text-sm font-medium cursor-pointer transition-colors"
            >{{ loading ? 'Searching...' : 'Search' }}</button>
          </form>
        </div>
      </div>
    </div>

    <!-- Loading -->
    <div v-if="loading" class="text-center py-12">
      <div class="inline-block w-6 h-6 border-2 border-gray-600 border-t-white rounded-full animate-spin"></div>
      <p class="text-gray-400 text-sm mt-3">Searching...</p>
    </div>

    <!-- Error -->
    <div v-if="error" class="bg-red-950 border border-red-800 rounded-lg p-4">
      <p class="text-red-400 text-sm">{{ error }}</p>
    </div>

    <!-- ========= ATTESTATION DETAIL (from hash lookup) ========= -->
    <div v-if="attestation && signerConfig" class="space-y-4">
      <!-- Verdict banner -->
      <div v-if="verdictConfig" :class="['rounded-lg border p-4 flex items-center justify-between', verdictConfig.bg, verdictConfig.border]">
        <div>
          <span class="text-xs text-gray-500 uppercase tracking-wider">Verdict</span>
          <div class="flex items-center gap-3 mt-1">
            <span :class="['text-2xl font-bold tracking-wide', verdictConfig.color]">{{ verdictConfig.label }}</span>
            <span class="text-xs text-gray-500 bg-gray-800/60 px-2 py-0.5 rounded">{{ attestation.proof_type === 'zk_groth16' ? 'ZK Proof' : 'Verified by R3L' }}</span>
          </div>
        </div>
        <div class="flex items-center gap-4 text-xs">
          <span v-if="attestation.has_c2pa" class="text-green-400">C2PA</span>
          <span v-if="verdict?.identity?.wallet_verified_onchain" class="text-purple-400">Wallet on-chain</span>
        </div>
      </div>

      <!-- Content hash hero -->
      <div class="bg-gray-900 rounded-lg border border-gray-800 p-4">
        <div class="flex items-center justify-between mb-2">
          <span class="text-xs text-gray-500 uppercase tracking-wider">Content Hash</span>
          <div class="flex items-center gap-2">
            <span class="text-xs font-medium px-2 py-0.5 rounded" :class="[contentTypeBadge(attestation.content_type).color, contentTypeBadge(attestation.content_type).bg]">
              {{ contentTypeBadge(attestation.content_type).label }}
            </span>
            <button @click="copyToClipboard(attestation.content_hash, 'hash')" class="text-gray-500 hover:text-gray-300 cursor-pointer">
              <span v-if="copiedField === 'hash'" class="text-green-400 text-xs">Copied!</span>
              <svg v-else xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                <rect x="9" y="9" width="13" height="13" rx="2" ry="2" /><path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1" />
              </svg>
            </button>
          </div>
        </div>
        <p class="text-sm font-mono text-gray-200 break-all">{{ attestation.content_hash }}</p>
        <a v-if="attestation.source_url" :href="attestation.source_url" target="_blank" rel="noopener" class="text-xs text-blue-400 hover:text-blue-300 mt-2 inline-block truncate max-w-full">
          {{ attestation.source_url }}
        </a>
      </div>

      <!-- Image preview for stored images -->
      <div v-if="isPreviewableImage(attestation)" class="bg-gray-900 rounded-lg border border-gray-800 p-4">
        <img :src="getContentUrl(attestation.content_hash)" alt="Stored content" class="max-h-64 rounded mx-auto" />
      </div>

      <!-- Details table -->
      <div class="bg-gray-900 rounded-lg border border-gray-800 divide-y divide-gray-800">
        <div class="flex px-4 py-3">
          <span class="text-gray-400 w-44 shrink-0 text-sm">Timestamp</span>
          <span class="text-sm text-gray-100">{{ new Date(attestation.timestamp * 1000).toISOString() }}</span>
        </div>
        <div class="flex px-4 py-3">
          <span class="text-gray-400 w-44 shrink-0 text-sm">Submitted By</span>
          <span class="text-sm font-mono text-gray-300 break-all">{{ attestation.submitted_by }}</span>
        </div>
        <div v-if="attestation.email_domain" class="flex px-4 py-3">
          <span class="text-gray-400 w-44 shrink-0 text-sm">Email Domain</span>
          <span class="text-sm text-yellow-400 font-medium">{{ attestation.email_domain }}</span>
        </div>
        <div v-if="attestation.wallet_pubkey" class="flex px-4 py-3">
          <span class="text-gray-400 w-44 shrink-0 text-sm">Wallet</span>
          <div class="flex-1">
            <span class="text-sm font-mono text-purple-400 break-all">{{ attestation.wallet_pubkey }}</span>
            <span v-if="attestation.wallet_sig" class="ml-2 text-xs text-green-400">Verified on-chain</span>
          </div>
        </div>

        <!-- C2PA section (only if present) -->
        <div v-if="attestation.has_c2pa" class="flex px-4 py-3">
          <span class="text-gray-400 w-44 shrink-0 text-sm">C2PA Signer</span>
          <span :class="['text-sm font-medium', signerConfig.color]">{{ signerConfig.label }}</span>
        </div>
        <div v-if="sourceTypeLabel" class="flex px-4 py-3">
          <span class="text-gray-400 w-44 shrink-0 text-sm">Source Type</span>
          <span :class="['text-sm', sourceTypeColor]">{{ sourceTypeLabel }}</span>
        </div>
        <div v-if="attestation.issuer" class="flex px-4 py-3">
          <span class="text-gray-400 w-44 shrink-0 text-sm">Issuer</span>
          <span class="text-sm text-gray-100">{{ attestation.issuer }}</span>
        </div>
        <div v-if="attestation.common_name" class="flex px-4 py-3">
          <span class="text-gray-400 w-44 shrink-0 text-sm">Common Name</span>
          <span class="text-sm text-gray-100">{{ attestation.common_name }}</span>
        </div>
        <div v-if="attestation.software_agent" class="flex px-4 py-3">
          <span class="text-gray-400 w-44 shrink-0 text-sm">Software Agent</span>
          <span class="text-sm text-gray-100">{{ attestation.software_agent }}</span>
        </div>
        <div v-if="attestation.signing_time" class="flex px-4 py-3">
          <span class="text-gray-400 w-44 shrink-0 text-sm">Signing Time</span>
          <span class="text-sm text-gray-100">{{ attestation.signing_time }}</span>
        </div>

        <div v-if="attestation.verifier_version" class="flex px-4 py-3">
          <span class="text-gray-400 w-44 shrink-0 text-sm">Verifier Version</span>
          <span class="text-sm text-gray-300">{{ attestation.verifier_version }}</span>
        </div>
        <div v-if="attestation.trust_bundle_hash" class="flex px-4 py-3">
          <span class="text-gray-400 w-44 shrink-0 text-sm">Trust Bundle</span>
          <span class="text-sm font-mono text-gray-500 break-all">{{ attestation.trust_bundle_hash }}</span>
        </div>
      </div>
    </div>

    <!-- Not found for exact lookup -->
    <p v-if="notFound && !loading" class="text-gray-500 text-sm">No attestation found for this hash.</p>

    <!-- ========= SIMILARITY RESULTS ========= -->
    <div v-if="similarResult && !loading" class="space-y-4">
      <div class="flex items-center justify-between">
        <h2 class="text-lg font-semibold">
          {{ similarResult.matches.length }} similar match{{ similarResult.matches.length !== 1 ? 'es' : '' }}
        </h2>
        <div class="text-right">
          <span class="text-xs text-gray-500 font-mono">{{ truncHash(similarResult.query_hash) }}</span>
          <div v-if="similarResult.query_tlsh" class="text-xs text-gray-600 font-mono mt-0.5">TLSH: {{ similarResult.query_tlsh.slice(0, 20) }}...</div>
        </div>
      </div>

      <div v-if="similarResult.matches.length === 0" class="border border-gray-800 rounded-lg p-8 text-center">
        <p class="text-gray-400">No similar content found in the attestation database.</p>
      </div>

      <div class="space-y-3">
        <div
          v-for="match in similarResult.matches"
          :key="match.content_hash"
          class="border rounded-lg p-4 cursor-pointer hover:bg-gray-900/50 transition-colors"
          :class="matchConfig[match.match_type]?.border || 'border-gray-700'"
          @click="viewDetail(match.content_hash)"
        >
          <div class="flex items-start justify-between gap-4">
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-3 mb-2">
                <span
                  class="text-xs font-medium px-2 py-0.5 rounded"
                  :class="[matchConfig[match.match_type]?.color, matchConfig[match.match_type]?.bg]"
                >{{ matchConfig[match.match_type]?.label }}</span>
                <span v-if="match.content_type && match.content_type !== 'file'" class="text-xs font-medium px-1.5 py-0.5 rounded" :class="[contentTypeBadge(match.content_type).color, contentTypeBadge(match.content_type).bg]">
                  {{ contentTypeBadge(match.content_type).label }}
                </span>
                <span class="text-xs text-gray-500 font-mono truncate">{{ truncHash(match.content_hash) }}</span>
              </div>
              <div v-if="match.tlsh_hash" class="text-xs text-gray-600 font-mono mb-1 truncate">
                TLSH: {{ match.tlsh_hash }}
              </div>
              <div class="flex flex-wrap gap-4 text-xs text-gray-400">
                <span v-if="match.issuer" class="truncate max-w-48">{{ match.issuer }}</span>
                <span v-if="match.trust_list_match" class="text-green-500">{{ match.trust_list_match }}</span>
                <span v-if="match.has_c2pa" class="text-blue-400">C2PA</span>
                <span v-if="match.timestamp">{{ formatTime(match.timestamp) }}</span>
              </div>
            </div>
            <div class="flex flex-col items-end gap-1 shrink-0">
              <div v-if="match.clip_similarity != null" class="text-right">
                <div class="text-xs text-gray-500">Visual</div>
                <div class="text-sm font-mono" :class="match.clip_similarity >= 0.9 ? 'text-green-400' : match.clip_similarity >= 0.8 ? 'text-blue-400' : 'text-yellow-400'">
                  {{ (match.clip_similarity * 100).toFixed(1) }}%
                </div>
              </div>
              <div v-if="match.tlsh_distance != null" class="text-right">
                <div class="text-xs text-gray-500">TLSH dist</div>
                <div class="text-sm font-mono" :class="match.tlsh_distance <= 50 ? 'text-green-400' : match.tlsh_distance <= 100 ? 'text-blue-400' : 'text-yellow-400'">
                  {{ match.tlsh_distance }}
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- ========= ALL ATTESTATIONS LIST (default view) ========= -->
    <div v-if="!showingDetail && !similarResult && !loading && !notFound" class="space-y-3">
      <h2 class="text-lg font-semibold text-gray-300">All Attestations</h2>

      <div v-if="listLoading" class="text-gray-500 text-sm py-8 text-center">Loading attestations...</div>

      <div v-else-if="allAttestations.length === 0" class="text-gray-500 text-sm py-8 text-center">
        No attestations found.
      </div>

      <div v-else class="bg-gray-900 rounded-lg border border-gray-800 divide-y divide-gray-800">
        <button
          v-for="item in allAttestations"
          :key="item.content_hash"
          @click="selectItem(item)"
          class="w-full flex items-center gap-4 px-4 py-3 hover:bg-gray-800/50 transition-colors text-left cursor-pointer"
        >
          <div class="flex-1 min-w-0">
            <div class="flex items-center gap-2">
              <span class="text-xs font-medium px-1.5 py-0.5 rounded" :class="[contentTypeBadge(item.content_type).color, contentTypeBadge(item.content_type).bg]">
                {{ contentTypeBadge(item.content_type).label }}
              </span>
              <span class="text-sm font-mono text-gray-300">{{ truncHash(item.content_hash) }}</span>
              <span v-if="item.email_domain" class="text-xs text-yellow-400">{{ item.email_domain }}</span>
              <span v-if="item.wallet_pubkey" class="text-xs text-purple-400 font-mono">{{ item.wallet_pubkey.slice(0, 4) }}...{{ item.wallet_pubkey.slice(-4) }}</span>
            </div>
            <div class="text-xs text-gray-500 mt-0.5">
              <span v-if="item.source_url" class="truncate max-w-xs inline-block align-middle">{{ item.source_url }}</span>
              <span v-else-if="item.issuer">{{ item.issuer }}</span>
              <span v-else-if="item.trust_list_match">{{ item.trust_list_match }}</span>
              <span v-else>{{ item.proof_type }}</span>
            </div>
          </div>
          <span class="text-xs text-gray-600 shrink-0">{{ formatTime(item.timestamp) }}</span>
        </button>
      </div>
    </div>
  </div>
</template>
