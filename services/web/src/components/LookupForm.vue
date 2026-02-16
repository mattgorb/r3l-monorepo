<script setup lang="ts">
import { ref, computed } from 'vue'
import type { AttestationResponse } from '../types'
import { lookupAttestation } from '../api'

const hash = ref('')
const loading = ref(false)
const error = ref<string | null>(null)
const attestation = ref<AttestationResponse | null>(null)
const notFound = ref(false)

async function search() {
  if (!hash.value.trim()) return
  loading.value = true
  error.value = null
  attestation.value = null
  notFound.value = false
  try {
    attestation.value = await lookupAttestation(hash.value.trim())
  } catch (e: any) {
    if (e.response?.status === 404) {
      notFound.value = true
    } else {
      error.value = e.response?.data || e.message
    }
  } finally {
    loading.value = false
  }
}

// --- Signer status from on-chain trust_list_match ---
const signerConfig = computed(() => {
  if (!attestation.value) return null
  const match = attestation.value.trust_list_match
  const configs: Record<string, { label: string; color: string; bg: string; desc: string }> = {
    official: {
      label: 'Trusted (C2PA Official)',
      color: 'text-green-400',
      bg: 'bg-green-950 border-green-800',
      desc: 'Signed by a certificate on the C2PA official trust list.',
    },
    curated: {
      label: 'Trusted (r3l Curated)',
      color: 'text-green-400',
      bg: 'bg-green-950 border-green-800',
      desc: 'Signed by a certificate on the r3l curated trust list.',
    },
    untrusted: {
      label: 'Unknown Signer',
      color: 'text-yellow-400',
      bg: 'bg-yellow-950 border-yellow-800',
      desc: 'C2PA metadata present but the signing certificate is not recognized by C2PA.org or r3l.',
    },
  }
  return configs[match] || {
    label: match || 'Unknown',
    color: 'text-gray-400',
    bg: 'bg-gray-900 border-gray-700',
    desc: '',
  }
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
  const raw = attestation.value?.digital_source_type
  if (!raw) return null
  return sourceTypeMap[raw] || raw
})

const sourceTypeColor = computed(() => {
  const raw = attestation.value?.digital_source_type || ''
  if (raw.includes('trainedAlgorithmicMedia')) return 'text-purple-400'
  if (raw.includes('compositeWithTrainedAlgorithmicMedia')) return 'text-purple-400'
  if (raw.includes('algorithmicMedia')) return 'text-purple-400'
  if (raw.includes('digitalCapture')) return 'text-blue-400'
  return 'text-gray-300'
})
</script>

<template>
  <div>
    <h2 class="text-lg font-semibold mb-3">Lookup Attestation</h2>
    <form @submit.prevent="search" class="flex gap-2 mb-4">
      <input
        v-model="hash"
        placeholder="Content hash (hex)"
        class="flex-1 bg-gray-900 border border-gray-700 rounded px-3 py-2 text-sm font-mono focus:outline-none focus:border-blue-500"
      />
      <button
        type="submit"
        :disabled="loading"
        class="px-4 py-2 bg-blue-600 hover:bg-blue-500 disabled:opacity-50 rounded text-sm font-medium cursor-pointer"
      >
        {{ loading ? '...' : 'Search' }}
      </button>
    </form>

    <p v-if="notFound" class="text-gray-500 text-sm">No attestation found for this hash.</p>
    <p v-if="error" class="text-red-400 text-sm">{{ error }}</p>

    <div v-if="attestation && signerConfig" class="space-y-4">
      <!-- Signer status banner -->
      <div :class="['rounded-lg border p-4', signerConfig.bg]">
        <span :class="['text-base font-semibold', signerConfig.color]">{{ signerConfig.label }}</span>
        <p v-if="signerConfig.desc" class="text-sm text-gray-400 mt-1">{{ signerConfig.desc }}</p>
      </div>

      <!-- Source Type -->
      <div v-if="sourceTypeLabel" class="bg-gray-900 rounded-lg border border-gray-800 p-4">
        <span class="text-gray-400 text-sm">Source Type</span>
        <p :class="['text-base font-medium mt-1', sourceTypeColor]">{{ sourceTypeLabel }}</p>
        <p v-if="attestation.digital_source_type" class="text-xs text-gray-600 mt-1 break-all">{{ attestation.digital_source_type }}</p>
      </div>

      <!-- Details -->
      <div class="bg-gray-900 rounded-lg border border-gray-800 divide-y divide-gray-800">
        <div class="flex px-4 py-3">
          <span class="text-gray-400 w-44 shrink-0 text-sm">Content Hash</span>
          <span class="text-sm font-mono text-gray-300 break-all">{{ attestation.content_hash }}</span>
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
        <div class="flex px-4 py-3">
          <span class="text-gray-400 w-44 shrink-0 text-sm">Submitted By</span>
          <span class="text-sm font-mono text-gray-300 break-all">{{ attestation.submitted_by }}</span>
        </div>
        <div class="flex px-4 py-3">
          <span class="text-gray-400 w-44 shrink-0 text-sm">Timestamp</span>
          <span class="text-sm text-gray-100">{{ new Date(attestation.timestamp * 1000).toISOString() }}</span>
        </div>
      </div>
    </div>
  </div>
</template>
