<script setup lang="ts">
import { ref } from 'vue'
import type { VerifyOutput } from '../types'
import { submitAttestation } from '../api'

const props = defineProps<{
  result: VerifyOutput
  proof: string | null
  publicInputs: string | null
}>()

const loading = ref(false)
const error = ref<string | null>(null)
const signature = ref<string | null>(null)
const pda = ref<string | null>(null)

async function submit() {
  loading.value = true
  error.value = null
  try {
    const resp = await submitAttestation({
      content_hash: props.result.content_hash!,
      has_c2pa: props.result.has_c2pa,
      trust_list_match: props.result.trust_list_match || '',
      validation_state: props.result.validation_state || '',
      digital_source_type: props.result.digital_source_type || '',
      issuer: props.result.issuer || '',
      common_name: props.result.common_name || '',
      software_agent: props.result.software_agent || '',
      signing_time: props.result.signing_time || '',
      proof: props.proof || undefined,
      public_inputs: props.publicInputs || undefined,
    })
    signature.value = resp.signature
    pda.value = resp.attestation_pda
  } catch (e: any) {
    error.value = e.response?.data || e.message
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div class="bg-gray-900 rounded-lg border border-gray-800 p-4">
    <div class="flex items-center justify-between">
      <div>
        <h3 class="font-medium text-sm">Submit to Solana</h3>
        <p class="text-xs text-gray-500">Store this attestation on-chain</p>
      </div>
      <button
        v-if="!signature"
        @click="submit"
        :disabled="loading"
        class="px-4 py-2 bg-purple-600 hover:bg-purple-500 disabled:opacity-50 rounded text-sm font-medium cursor-pointer"
      >
        {{ loading ? 'Submitting...' : 'Submit' }}
      </button>
      <span v-else class="text-green-400 text-sm">Stored</span>
    </div>
    <div v-if="signature" class="mt-3 space-y-1">
      <p class="text-xs text-gray-500">
        Tx: <span class="font-mono text-gray-300">{{ signature }}</span>
      </p>
      <p class="text-xs text-gray-500">
        PDA: <span class="font-mono text-gray-300">{{ pda }}</span>
      </p>
    </div>
    <p v-if="error" class="mt-2 text-red-400 text-xs">{{ error }}</p>
  </div>
</template>
