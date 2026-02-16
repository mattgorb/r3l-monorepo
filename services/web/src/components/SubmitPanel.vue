<script setup lang="ts">
import { ref, computed } from 'vue'
import type { VerifyOutput } from '../types'
import { submitAttestation } from '../api'

const props = defineProps<{
  result: VerifyOutput
  proof: string | null
  publicInputs: string | null
  disabled?: boolean
}>()

const emit = defineEmits<{
  submitted: [signature: string, pda: string]
}>()

const loading = ref(false)
const error = ref<string | null>(null)
const done = ref(false)

// Clean up API error messages
function cleanError(raw: string): string {
  const cleaned = raw
    .replace(/--- stdout ---\n?/g, '')
    .replace(/--- stderr ---\n?/g, '')
    .trim()
  const lines = cleaned.split('\n').filter(l => l.trim())
  if (lines.length > 3) {
    return lines.slice(0, 3).join('\n')
  }
  return cleaned
}

async function submit() {
  if (props.disabled) return
  loading.value = true
  error.value = null
  try {
    const resp = await submitAttestation({
      content_hash: props.result.content_hash!,
      proof: props.proof || '',
      public_inputs: props.publicInputs || '',
    })
    done.value = true
    emit('submitted', resp.signature, resp.attestation_pda)
  } catch (e: any) {
    const raw = e.response?.data || e.message || 'Unknown error'
    error.value = cleanError(typeof raw === 'string' ? raw : JSON.stringify(raw))
  } finally {
    loading.value = false
  }
}

const buttonDisabled = computed(() => props.disabled || loading.value || done.value)
</script>

<template>
  <div>
    <div class="flex items-center justify-between">
      <div>
        <h3 class="font-medium text-sm">Submit to Solana</h3>
        <p class="text-xs text-gray-500">Store this attestation on-chain</p>
      </div>
      <button
        v-if="!done"
        @click="submit"
        :disabled="buttonDisabled"
        :class="[
          'px-4 py-2 rounded text-sm font-medium',
          buttonDisabled
            ? 'bg-gray-700 text-gray-500 cursor-not-allowed'
            : 'bg-purple-600 hover:bg-purple-500 cursor-pointer'
        ]"
      >
        {{ loading ? 'Submitting...' : 'Submit' }}
      </button>
      <span v-else class="text-green-400 text-sm font-medium">Stored</span>
    </div>
    <p v-if="error" class="mt-2 text-red-400 text-xs whitespace-pre-line">{{ error }}</p>
  </div>
</template>
