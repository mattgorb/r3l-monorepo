<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import type { VerifyOutput, AttestResponse, MeResponse } from '../types'
import FileUpload from '../components/FileUpload.vue'
import { attestFile, getMe } from '../api'

const result = ref<VerifyOutput | null>(null)
const file = ref<File | null>(null)

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
  // Auto-populate wallet if logged in with wallet
  if (loggedInUser.value?.wallet_pubkey) {
    walletPubkey.value = loggedInUser.value.wallet_pubkey
    walletConnected.value = true
    if (hasPhantom.value) {
      walletMode.value = 'phantom'
    } else {
      walletMode.value = 'manual'
      manualPubkey.value = loggedInUser.value.wallet_pubkey
    }
  }
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
    no_c2pa: { label: 'No C2PA', color: 'text-gray-400', borderColor: 'border-gray-700', desc: 'No C2PA provenance metadata found in this file.' },
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

// --- Wallet identity ---
type WalletMode = 'choose' | 'phantom' | 'manual'
const walletMode = ref<WalletMode>('choose')
const walletConnected = ref(false)
const walletPubkey = ref<string | null>(null)
const walletSigned = ref(false)
const walletSignatureValue = ref<string | null>(null)
const walletLoading = ref(false)
const walletError = ref<string | null>(null)
const manualPubkey = ref('')
const manualSignature = ref('')

const walletMessage = computed(() => {
  if (!result.value?.content_hash) return ''
  return `R3L: attest ${result.value.content_hash}`
})

const hasPhantom = computed(() => !!(window as any).solana?.isPhantom)

async function connectPhantom() {
  walletError.value = null
  const provider = (window as any).solana
  if (!provider?.isPhantom) {
    walletError.value = 'Phantom wallet not found.'
    return
  }
  try {
    const resp = await provider.connect()
    walletPubkey.value = resp.publicKey.toString()
    walletConnected.value = true
  } catch (e: any) {
    walletError.value = e.message || 'Failed to connect wallet'
  }
}

async function signWithPhantom() {
  if (!walletPubkey.value || !result.value?.content_hash) return
  walletLoading.value = true
  walletError.value = null

  const provider = (window as any).solana
  const message = walletMessage.value

  try {
    const encoded = new TextEncoder().encode(message)
    const { signature } = await provider.signMessage(encoded, 'utf8')
    const bs58 = await import('bs58')
    walletSignatureValue.value = bs58.default.encode(signature)
    walletSigned.value = true
  } catch (e: any) {
    walletError.value = e.message || 'Failed to sign message'
  } finally {
    walletLoading.value = false
  }
}

function submitManualWallet() {
  if (!manualPubkey.value || !manualSignature.value) return
  walletPubkey.value = manualPubkey.value.trim()
  walletSignatureValue.value = manualSignature.value.trim()
  walletSigned.value = true
}

// --- Single attest button (submits C2PA + wallet in one call) ---
const attesting = ref(false)
const attestError = ref<string | null>(null)
const attestResult = ref<AttestResponse | null>(null)

async function attestAll() {
  if (!file.value) return
  attesting.value = true
  attestError.value = null

  const opts: {
    walletPubkey?: string
    walletMessage?: string
    walletSignature?: string
  } = {}

  // Include wallet if signed
  if (walletSigned.value && walletPubkey.value && walletSignatureValue.value) {
    opts.walletPubkey = walletPubkey.value
    opts.walletMessage = walletMessage.value
    opts.walletSignature = walletSignatureValue.value
  }

  try {
    attestResult.value = await attestFile(file.value, opts)
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
  attestResult.value = null
  attestError.value = null
  attesting.value = false
  walletMode.value = 'choose'
  walletConnected.value = false
  walletPubkey.value = null
  walletSigned.value = false
  walletSignatureValue.value = null
  walletError.value = null
  manualPubkey.value = ''
  manualSignature.value = ''
}

onMounted(loadIdentity)
</script>

<template>
  <div class="space-y-6">
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-bold">Verify & Attest</h1>
      <button v-if="result" @click="reset" class="text-sm text-gray-400 hover:text-white transition-colors cursor-pointer">
        Upload another file
      </button>
    </div>

    <!-- Identity bar -->
    <div v-if="loggedInUser" class="bg-gray-900 rounded-lg border border-gray-800 px-4 py-2.5 flex items-center justify-between">
      <div class="flex items-center gap-2 text-sm">
        <span class="text-gray-500">Attesting as</span>
        <span class="text-gray-200 font-medium">{{ identityLabel }}</span>
        <span class="text-xs text-gray-600">({{ loggedInUser.type === 'org' ? 'org' : loggedInUser.auth_method }})</span>
      </div>
      <router-link to="/account" class="text-xs text-gray-500 hover:text-gray-300 transition-colors">Account</router-link>
    </div>
    <div v-else class="bg-gray-900/50 rounded-lg border border-gray-800/50 px-4 py-2.5 flex items-center justify-between">
      <span class="text-sm text-gray-600">Not logged in — attestations will be anonymous.</span>
      <router-link to="/account" class="text-xs text-gray-500 hover:text-gray-300 transition-colors">Sign in</router-link>
    </div>

    <!-- Upload -->
    <FileUpload v-if="!result" @verified="onVerified" />

    <!-- Results -->
    <template v-if="result">
      <!-- File name -->
      <div class="flex items-center gap-3">
        <span class="text-sm text-gray-500">{{ file?.name }}</span>
      </div>

      <!-- Report card -->
      <div :class="['bg-gray-900 rounded-lg border overflow-hidden', signerConfig.borderColor]">
        <div class="px-4 py-3 border-b border-gray-800">
          <span :class="['text-sm font-semibold', signerConfig.color]">{{ signerConfig.label }}</span>
          <p class="text-xs text-gray-500 mt-0.5">{{ signerConfig.desc }}</p>
        </div>

        <div class="divide-y divide-gray-800">
          <div v-if="result.format" class="flex items-center px-4 py-3">
            <span class="text-gray-400 w-44 shrink-0 text-sm">Media Type</span>
            <span class="text-sm text-gray-100">{{ result.format }}</span>
          </div>
          <div v-if="sourceTypeLabel" class="flex px-4 py-3">
            <span class="text-gray-400 w-44 shrink-0 text-sm">Source Type</span>
            <div>
              <span :class="['text-sm font-medium', sourceTypeColor]">{{ sourceTypeLabel }}</span>
              <p class="text-xs text-gray-600 mt-0.5 break-all">{{ result.digital_source_type }}</p>
            </div>
          </div>
          <div v-if="result.content_hash" class="flex items-center px-4 py-3">
            <span class="text-gray-400 w-44 shrink-0 text-sm">Content Hash</span>
            <span class="text-sm font-mono text-gray-300 break-all flex-1">{{ result.content_hash }}</span>
            <button @click="copyToClipboard(result.content_hash!, 'hash')" class="shrink-0 ml-2 text-gray-500 hover:text-gray-300 cursor-pointer">
              <span v-if="copiedField === 'hash'" class="text-green-400 text-xs">Copied</span>
              <svg v-else xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                <rect x="9" y="9" width="13" height="13" rx="2" ry="2" /><path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1" />
              </svg>
            </button>
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
      </div>

      <!-- ═══ Optional: Wallet Identity ═══ -->
      <div v-if="!attestResult" class="bg-gray-900 rounded-lg border border-gray-800 p-5 space-y-3">
        <div>
          <p class="text-sm font-semibold text-gray-100">Wallet Identity <span class="text-xs font-normal text-gray-500">(optional)</span></p>
          <p class="text-xs text-gray-500 mt-1">Bind your Solana wallet pubkey to this file.</p>
        </div>

        <!-- Mode chooser -->
        <template v-if="walletMode === 'choose' && !walletSigned">
          <button
            v-if="hasPhantom"
            @click="walletMode = 'phantom'"
            class="w-full py-2.5 bg-purple-600 hover:bg-purple-500 rounded-lg text-sm font-medium transition-colors cursor-pointer"
          >
            Connect Phantom Wallet
          </button>
          <button
            @click="walletMode = 'manual'"
            :class="[
              'w-full py-2.5 rounded-lg text-sm font-medium transition-colors cursor-pointer',
              hasPhantom ? 'bg-gray-800 hover:bg-gray-700 text-gray-300' : 'bg-purple-600 hover:bg-purple-500 text-white',
            ]"
          >
            Enter Key Manually
          </button>
        </template>

        <!-- Phantom flow -->
        <template v-if="walletMode === 'phantom' && !walletSigned">
          <template v-if="!walletConnected">
            <button
              @click="connectPhantom"
              class="w-full py-2.5 bg-purple-600 hover:bg-purple-500 rounded-lg text-sm font-medium transition-colors cursor-pointer"
            >
              Connect Phantom
            </button>
          </template>
          <template v-else>
            <div class="flex items-center gap-2">
              <span class="text-green-400">&#10003;</span>
              <span class="text-sm text-gray-300">{{ walletPubkey?.slice(0, 4) }}...{{ walletPubkey?.slice(-4) }}</span>
            </div>
            <button
              @click="signWithPhantom"
              :disabled="walletLoading"
              :class="[
                'w-full py-2.5 rounded-lg text-sm font-medium transition-colors',
                !walletLoading ? 'bg-purple-600 hover:bg-purple-500 text-white cursor-pointer' : 'bg-gray-800 text-gray-500 cursor-not-allowed',
              ]"
            >
              {{ walletLoading ? 'Signing...' : 'Sign Message' }}
            </button>
          </template>
          <button @click="walletMode = 'choose'; walletConnected = false; walletPubkey = null" class="w-full text-xs text-gray-500 hover:text-gray-300 transition-colors cursor-pointer">Cancel</button>
        </template>

        <!-- Manual flow -->
        <template v-if="walletMode === 'manual' && !walletSigned">
          <input
            v-model="manualPubkey"
            type="text"
            placeholder="Pubkey (base58)"
            class="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-sm text-white font-mono placeholder-gray-600 focus:border-purple-500 focus:outline-none"
          />
          <div class="bg-gray-800 rounded-lg px-3 py-2">
            <p class="text-xs text-gray-500 mb-1">Sign this message with your wallet:</p>
            <p class="text-xs font-mono text-purple-300 break-all select-all">{{ walletMessage }}</p>
          </div>
          <input
            v-model="manualSignature"
            type="text"
            placeholder="Signature (base58)"
            class="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-sm text-white font-mono placeholder-gray-600 focus:border-purple-500 focus:outline-none"
          />
          <button
            @click="submitManualWallet"
            :disabled="!manualPubkey || !manualSignature"
            :class="[
              'w-full py-2.5 rounded-lg text-sm font-medium transition-colors',
              manualPubkey && manualSignature
                ? 'bg-purple-600 hover:bg-purple-500 text-white cursor-pointer'
                : 'bg-gray-800 text-gray-500 cursor-not-allowed',
            ]"
          >
            Confirm
          </button>
          <button @click="walletMode = 'choose'" class="w-full text-xs text-gray-500 hover:text-gray-300 transition-colors cursor-pointer">Cancel</button>
        </template>

        <div v-if="walletSigned" class="flex items-center gap-2">
          <span class="text-green-400">&#10003;</span>
          <span class="text-sm text-green-400 font-medium">Wallet {{ walletPubkey?.slice(0, 4) }}...{{ walletPubkey?.slice(-4) }} signed</span>
        </div>

        <p v-if="walletError" class="text-red-400 text-xs">{{ walletError }}</p>
      </div>

      <!-- ═══ Attest Button ═══ -->
      <div v-if="!attestResult" class="space-y-3">
        <button
          @click="attestAll"
          :disabled="attesting"
          class="w-full py-3 bg-blue-600 hover:bg-blue-500 disabled:opacity-50 rounded-lg text-sm font-medium transition-colors cursor-pointer"
        >
          {{ attesting ? 'Attesting on Solana...' : 'Attest on Solana' }}
        </button>
        <p class="text-xs text-gray-600 text-center">
          Stores a permanent on-chain record of this verification{{ walletSigned ? ' + wallet' : '' }}.
        </p>
        <p v-if="attestError" class="text-red-400 text-sm text-center">{{ attestError }}</p>
      </div>

      <!-- ═══ On-chain Results ═══ -->
      <div v-if="attestResult" class="space-y-4">
        <h2 class="text-lg font-semibold">On-chain Result</h2>

        <div class="bg-gray-900 rounded-lg border border-green-900 p-4 space-y-2">
          <div class="flex items-center gap-2">
            <span class="text-green-400">&#10003;</span>
            <span class="text-sm text-green-400 font-medium">
              {{ attestResult.existing ? 'Attestation already exists' : 'Attestation recorded on Solana' }}
            </span>
          </div>
          <div class="space-y-1">
            <div class="flex items-center gap-2">
              <span class="text-xs text-gray-500 w-20 shrink-0">Tx</span>
              <span class="text-xs font-mono text-gray-300 truncate flex-1">{{ attestResult.signature || '(existing)' }}</span>
              <button v-if="attestResult.signature" @click="copyToClipboard(attestResult.signature, 'tx')" class="shrink-0 text-gray-500 hover:text-gray-300 cursor-pointer">
                <span v-if="copiedField === 'tx'" class="text-green-400 text-xs">Copied</span>
                <span v-else class="text-xs">Copy</span>
              </button>
            </div>
            <div class="flex items-center gap-2">
              <span class="text-xs text-gray-500 w-20 shrink-0">PDA</span>
              <span class="text-xs font-mono text-gray-300 truncate flex-1">{{ attestResult.attestation_pda }}</span>
              <button @click="copyToClipboard(attestResult.attestation_pda, 'pda')" class="shrink-0 text-gray-500 hover:text-gray-300 cursor-pointer">
                <span v-if="copiedField === 'pda'" class="text-green-400 text-xs">Copied</span>
                <span v-else class="text-xs">Copy</span>
              </button>
            </div>
            <div v-if="attestResult.wallet_pubkey" class="flex items-center gap-2">
              <span class="text-xs text-gray-500 w-20 shrink-0">Wallet</span>
              <span class="text-xs font-mono text-purple-400">{{ attestResult.wallet_pubkey }}</span>
            </div>
          </div>
        </div>

        <router-link
          :to="'/search/' + attestResult.content_hash"
          class="inline-block text-sm text-blue-400 hover:text-blue-300 transition-colors"
        >
          View attestation &rarr;
        </router-link>
      </div>
    </template>
  </div>
</template>
