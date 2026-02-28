<script setup lang="ts">
import { ref, computed } from 'vue'
import type { VerifyOutput } from '../types'
import ProofStatus from './ProofStatus.vue'
import SubmitPanel from './SubmitPanel.vue'

const props = defineProps<{
  result: VerifyOutput
  file: File
}>()

// --- Signer status derived from validation_codes ---
type SignerStatus = 'trusted_official' | 'trusted_curated' | 'unknown_signer' | 'revoked' | 'expired' | 'self_signed' | 'no_c2pa'

const signerStatus = computed<SignerStatus>(() => {
  if (!props.result.has_c2pa) return 'no_c2pa'
  const codes = props.result.validation_codes || []
  if (codes.includes('signingCredential.revoked')) return 'revoked'
  if (codes.includes('signingCredential.expired')) return 'expired'
  if (codes.includes('signingCredential.untrusted')) {
    if (props.result.issuer && props.result.common_name
      && props.result.issuer === props.result.common_name) {
      return 'self_signed'
    }
    return 'unknown_signer'
  }
  if (props.result.trust_list_match === 'official') return 'trusted_official'
  if (props.result.trust_list_match === 'curated') return 'trusted_curated'
  return 'unknown_signer'
})

const signerConfig = computed(() => {
  const configs: Record<SignerStatus, { label: string; color: string; borderColor: string; desc: string }> = {
    trusted_official: {
      label: 'Trusted (C2PA Official)',
      color: 'text-green-400',
      borderColor: 'border-green-800',
      desc: 'Signed by a certificate on the C2PA official trust list.',
    },
    trusted_curated: {
      label: 'Trusted (r3l Curated)',
      color: 'text-green-400',
      borderColor: 'border-green-800',
      desc: 'Signed by a certificate on the r3l curated trust list.',
    },
    unknown_signer: {
      label: 'Unknown Signer',
      color: 'text-yellow-400',
      borderColor: 'border-yellow-800',
      desc: 'C2PA metadata present but the signing certificate is not recognized by C2PA.org or r3l.',
    },
    revoked: {
      label: 'Revoked Certificate',
      color: 'text-red-400',
      borderColor: 'border-red-800',
      desc: 'The signing certificate has been revoked.',
    },
    expired: {
      label: 'Expired Certificate',
      color: 'text-orange-400',
      borderColor: 'border-orange-800',
      desc: 'The signing certificate has expired.',
    },
    self_signed: {
      label: 'Self-Signed',
      color: 'text-red-400',
      borderColor: 'border-red-800',
      desc: 'Signed with a self-signed certificate. Cannot verify origin.',
    },
    no_c2pa: {
      label: 'No C2PA',
      color: 'text-gray-400',
      borderColor: 'border-gray-700',
      desc: 'No embedded C2PA metadata found in this file.',
    },
  }
  return configs[signerStatus.value]
})

// --- Human-readable source type ---
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
  const raw = props.result.digital_source_type
  if (!raw) return null
  return sourceTypeMap[raw] || raw
})

const sourceTypeColor = computed(() => {
  const raw = props.result.digital_source_type || ''
  if (raw.includes('trainedAlgorithmicMedia')) return 'text-purple-400'
  if (raw.includes('compositeWithTrainedAlgorithmicMedia')) return 'text-purple-400'
  if (raw.includes('algorithmicMedia')) return 'text-purple-400'
  if (raw.includes('digitalCapture')) return 'text-blue-400'
  return 'text-gray-100'
})

// --- Ingredients and actions ---
const ingredients = computed(() => {
  const raw = props.result.ingredients
  if (!raw || !Array.isArray(raw)) return []
  return raw.map((ing: any) => ({
    title: ing.title || 'Unknown',
    format: ing.format || null,
    relationship: ing.relationship || null,
  }))
})

const actions = computed(() => {
  const raw = props.result.actions
  if (!raw || !Array.isArray(raw)) return []
  return raw.map((a: any) => {
    const action = typeof a === 'string' ? a : (a.action || 'unknown')
    // Clean up c2pa. prefix for readability
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
  } catch {
    // Fallback: no-op
  }
}

// --- Step flow ---
const proofHex = ref<string | null>(null)
const publicInputsHex = ref<string | null>(null)
const proofDone = ref(false)
const submitDone = ref(false)
const txSignature = ref<string | null>(null)
const txPda = ref<string | null>(null)

function onProved(proof: string, publicInputs: string) {
  proofHex.value = proof
  publicInputsHex.value = publicInputs
  proofDone.value = true
}

function onSubmitted(sig: string, pda: string) {
  txSignature.value = sig
  txPda.value = pda
  submitDone.value = true
}
</script>

<template>
  <div class="space-y-6">
    <!-- Verification report card -->
    <div :class="['bg-gray-900 rounded-lg border overflow-hidden', signerConfig.borderColor]">
      <!-- Header -->
      <div class="px-4 py-3 border-b border-gray-800">
        <span :class="['text-sm font-semibold', signerConfig.color]">{{ signerConfig.label }}</span>
        <p class="text-xs text-gray-500 mt-0.5">{{ signerConfig.desc }}</p>
      </div>

      <!-- All fields -->
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
          <button
            @click="copyToClipboard(result.content_hash!, 'hash')"
            class="shrink-0 ml-2 text-gray-500 hover:text-gray-300 cursor-pointer"
          >
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
            <span
              v-for="code in result.validation_codes"
              :key="code"
              class="text-xs font-mono px-2 py-0.5 rounded bg-gray-800 text-gray-300"
            >{{ code }}</span>
          </div>
        </div>
        <!-- Ingredients (edit chain) -->
        <div v-if="ingredients.length > 0" class="px-4 py-3">
          <span class="text-gray-400 text-sm">Ingredients</span>
          <div class="mt-2 space-y-2">
            <div
              v-for="(ing, i) in ingredients"
              :key="i"
              class="flex items-center gap-3 bg-gray-800/50 rounded px-3 py-2"
            >
              <span class="text-xs text-gray-500 shrink-0">{{ ing.relationship || 'ingredient' }}</span>
              <span class="text-sm text-gray-200">{{ ing.title }}</span>
              <span v-if="ing.format" class="text-xs text-gray-500">{{ ing.format }}</span>
            </div>
          </div>
        </div>
        <!-- Actions (edit history) -->
        <div v-if="actions.length > 0" class="px-4 py-3">
          <span class="text-gray-400 text-sm">Edit History</span>
          <div class="mt-2 flex flex-wrap gap-1.5">
            <span
              v-for="(action, i) in actions"
              :key="i"
              class="text-xs px-2 py-0.5 rounded bg-gray-800 text-gray-300"
            >{{ action }}</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Pipeline Steps (only if C2PA found) -->
    <div v-if="result.has_c2pa && result.content_hash" class="space-y-3">
      <h3 class="text-sm font-medium text-gray-400 uppercase tracking-wide">Pipeline</h3>

      <!-- Step 1: Verify (always done) -->
      <div class="bg-gray-900 rounded-lg border border-gray-800 p-4">
        <div class="flex items-center gap-3">
          <span class="shrink-0 w-6 h-6 rounded-full bg-green-900 text-green-400 text-xs font-bold flex items-center justify-center">1</span>
          <div class="flex-1">
            <span class="text-sm font-medium text-green-400">Verified</span>
            <p class="text-xs text-gray-500">C2PA metadata verified successfully</p>
          </div>
        </div>
      </div>

      <!-- Step 2: ZK Proof -->
      <div class="bg-gray-900 rounded-lg border border-gray-800 p-4">
        <div class="flex items-center gap-3">
          <span :class="['shrink-0 w-6 h-6 rounded-full text-xs font-bold flex items-center justify-center',
            proofDone ? 'bg-green-900 text-green-400' : 'bg-gray-800 text-gray-400']">2</span>
          <div class="flex-1">
            <ProofStatus :file="file" @proved="onProved" />
          </div>
        </div>
      </div>

      <!-- Step 3: Submit to Solana -->
      <div :class="['bg-gray-900 rounded-lg border border-gray-800 p-4', proofDone ? '' : 'opacity-50']">
        <div class="flex items-center gap-3">
          <span :class="['shrink-0 w-6 h-6 rounded-full text-xs font-bold flex items-center justify-center',
            submitDone ? 'bg-green-900 text-green-400' : 'bg-gray-800 text-gray-400']">3</span>
          <div class="flex-1">
            <SubmitPanel
              :result="result"
              :proof="proofHex"
              :public-inputs="publicInputsHex"
              :disabled="!proofDone"
              @submitted="onSubmitted"
            />
          </div>
        </div>
      </div>

      <!-- On-chain result -->
      <div v-if="submitDone && txSignature" class="bg-gray-900 rounded-lg border border-green-900 p-4 space-y-2">
        <p class="text-sm font-medium text-green-400">Attestation stored on-chain</p>
        <div class="flex items-center gap-2">
          <span class="text-xs text-gray-500 w-10 shrink-0">Tx</span>
          <span class="text-xs font-mono text-gray-300 break-all flex-1">{{ txSignature }}</span>
          <button @click="copyToClipboard(txSignature!, 'tx')" class="shrink-0 text-gray-500 hover:text-gray-300 cursor-pointer">
            <span v-if="copiedField === 'tx'" class="text-green-400 text-xs">Copied</span>
            <svg v-else xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
              <rect x="9" y="9" width="13" height="13" rx="2" ry="2" /><path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1" />
            </svg>
          </button>
        </div>
        <div class="flex items-center gap-2">
          <span class="text-xs text-gray-500 w-10 shrink-0">PDA</span>
          <span class="text-xs font-mono text-gray-300 break-all flex-1">{{ txPda }}</span>
          <button @click="copyToClipboard(txPda!, 'pda')" class="shrink-0 text-gray-500 hover:text-gray-300 cursor-pointer">
            <span v-if="copiedField === 'pda'" class="text-green-400 text-xs">Copied</span>
            <svg v-else xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
              <rect x="9" y="9" width="13" height="13" rx="2" ry="2" /><path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1" />
            </svg>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
