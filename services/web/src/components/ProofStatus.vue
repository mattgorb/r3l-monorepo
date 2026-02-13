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
const proofPreview = ref('')

async function generate() {
  loading.value = true
  error.value = null
  try {
    const result = await proveFile(props.file)
    proofPreview.value = result.proof.slice(0, 64) + '...'
    emit('proved', result.proof, result.public_outputs)
    done.value = true
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
      <span v-else class="text-green-400 text-sm">Done</span>
    </div>
    <p v-if="done" class="mt-2 text-xs text-gray-500 font-mono break-all">{{ proofPreview }}</p>
    <p v-if="error" class="mt-2 text-red-400 text-xs">{{ error }}</p>
  </div>
</template>
