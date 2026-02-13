<script setup lang="ts">
import { ref } from 'vue'
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

const trustColor = (match: string) => {
  switch (match) {
    case 'official': return 'text-green-400'
    case 'curated': return 'text-yellow-400'
    case 'untrusted': return 'text-red-400'
    default: return 'text-gray-400'
  }
}
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

    <div v-if="attestation" class="bg-gray-900 rounded-lg border border-gray-800 divide-y divide-gray-800">
      <div class="flex px-4 py-3">
        <span class="text-gray-400 w-44 shrink-0 text-sm">Trust</span>
        <span :class="['text-sm font-medium', trustColor(attestation.trust_list_match)]">
          {{ attestation.trust_list_match }}
        </span>
      </div>
      <div class="flex px-4 py-3">
        <span class="text-gray-400 w-44 shrink-0 text-sm">Validation</span>
        <span class="text-sm">{{ attestation.validation_state }}</span>
      </div>
      <div v-if="attestation.issuer" class="flex px-4 py-3">
        <span class="text-gray-400 w-44 shrink-0 text-sm">Issuer</span>
        <span class="text-sm">{{ attestation.issuer }}</span>
      </div>
      <div v-if="attestation.common_name" class="flex px-4 py-3">
        <span class="text-gray-400 w-44 shrink-0 text-sm">Common Name</span>
        <span class="text-sm">{{ attestation.common_name }}</span>
      </div>
      <div v-if="attestation.software_agent" class="flex px-4 py-3">
        <span class="text-gray-400 w-44 shrink-0 text-sm">Software Agent</span>
        <span class="text-sm">{{ attestation.software_agent }}</span>
      </div>
      <div v-if="attestation.digital_source_type" class="flex px-4 py-3">
        <span class="text-gray-400 w-44 shrink-0 text-sm">Source Type</span>
        <span class="text-sm break-all">{{ attestation.digital_source_type }}</span>
      </div>
      <div v-if="attestation.signing_time" class="flex px-4 py-3">
        <span class="text-gray-400 w-44 shrink-0 text-sm">Signing Time</span>
        <span class="text-sm">{{ attestation.signing_time }}</span>
      </div>
      <div class="flex px-4 py-3">
        <span class="text-gray-400 w-44 shrink-0 text-sm">Submitted By</span>
        <span class="text-sm font-mono break-all">{{ attestation.submitted_by }}</span>
      </div>
      <div class="flex px-4 py-3">
        <span class="text-gray-400 w-44 shrink-0 text-sm">Timestamp</span>
        <span class="text-sm">{{ new Date(attestation.timestamp * 1000).toISOString() }}</span>
      </div>
    </div>
  </div>
</template>
