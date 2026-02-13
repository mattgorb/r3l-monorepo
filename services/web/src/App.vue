<script setup lang="ts">
import { ref } from 'vue'
import type { VerifyOutput } from './types'
import FileUpload from './components/FileUpload.vue'
import Results from './components/Results.vue'
import LookupForm from './components/LookupForm.vue'

const verifyResult = ref<VerifyOutput | null>(null)
const uploadedFile = ref<File | null>(null)

function onVerified(result: VerifyOutput, file: File) {
  verifyResult.value = result
  uploadedFile.value = file
}

function reset() {
  verifyResult.value = null
  uploadedFile.value = null
}
</script>

<template>
  <div class="min-h-screen bg-gray-950 text-gray-100">
    <div class="max-w-3xl mx-auto px-4 py-12">
      <h1 class="text-3xl font-bold mb-2">Provenance Attestation</h1>
      <p class="text-gray-400 mb-8">
        Verify C2PA media provenance and store attestations on Solana.
      </p>

      <div v-if="!verifyResult">
        <FileUpload @verified="onVerified" />
      </div>
      <div v-else>
        <button
          @click="reset"
          class="text-sm text-gray-400 hover:text-white mb-4 cursor-pointer"
        >
          &larr; Upload another file
        </button>
        <Results
          :result="verifyResult"
          :file="uploadedFile!"
        />
      </div>

      <hr class="border-gray-800 my-10" />
      <LookupForm />
    </div>
  </div>
</template>
