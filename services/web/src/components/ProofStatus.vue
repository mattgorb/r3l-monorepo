<script setup lang="ts">
import { ref } from 'vue'
import { proveFile } from '../api'

const props = defineProps<{ file: File }>()
const emit = defineEmits<{
  proved: [proof: string, publicInputs: string]
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

async function generate() {
  loading.value = true
  error.value = null
  try {
    const result = await proveFile(props.file)
    emit('proved', result.proof, result.public_outputs)
    done.value = true
  } catch (e: any) {
    const raw = e.response?.data || e.message || 'Unknown error'
    error.value = cleanError(typeof raw === 'string' ? raw : JSON.stringify(raw))
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div>
    <div class="flex items-center justify-between">
      <div>
        <h3 class="font-medium text-sm">ZK Proof</h3>
        <p class="text-xs text-gray-500">Generate a Groth16 proof of C2PA verification</p>
      </div>
      <button
        v-if="!done"
        @click="generate"
        :disabled="loading"
        class="px-4 py-2 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 rounded text-sm font-medium cursor-pointer"
      >
        {{ loading ? 'Generating...' : 'Generate Proof' }}
      </button>
      <span v-else class="text-green-400 text-sm font-medium">Done</span>
    </div>
    <p v-if="error" class="mt-2 text-red-400 text-xs whitespace-pre-line">{{ error }}</p>
  </div>
</template>
