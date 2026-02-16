<script setup lang="ts">
import { ref, computed } from 'vue'
import type { VerifyOutput } from './types'
import FileUpload from './components/FileUpload.vue'
import Results from './components/Results.vue'
import LookupForm from './components/LookupForm.vue'

const verifyResult = ref<VerifyOutput | null>(null)
const uploadedFile = ref<File | null>(null)
const filePreviewUrl = ref<string | null>(null)

function onVerified(result: VerifyOutput, file: File) {
  verifyResult.value = result
  uploadedFile.value = file
  // Generate thumbnail preview for images
  if (file.type.startsWith('image/')) {
    filePreviewUrl.value = URL.createObjectURL(file)
  } else {
    filePreviewUrl.value = null
  }
}

function reset() {
  if (filePreviewUrl.value) URL.revokeObjectURL(filePreviewUrl.value)
  verifyResult.value = null
  uploadedFile.value = null
  filePreviewUrl.value = null
}

const fileSize = computed(() => {
  if (!uploadedFile.value) return ''
  const bytes = uploadedFile.value.size
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
})

const fileExt = computed(() => {
  if (!uploadedFile.value) return ''
  const name = uploadedFile.value.name
  const dot = name.lastIndexOf('.')
  return dot >= 0 ? name.slice(dot + 1).toUpperCase() : ''
})
</script>

<template>
  <div class="min-h-screen bg-gray-950 text-gray-100">
    <div class="max-w-5xl mx-auto px-6 py-12">
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

        <!-- File preview -->
        <div class="flex items-center gap-4 mb-6 bg-gray-900 rounded-lg border border-gray-800 p-4">
          <div v-if="filePreviewUrl" class="shrink-0">
            <img :src="filePreviewUrl" :alt="uploadedFile!.name" class="w-16 h-16 object-cover rounded" />
          </div>
          <div v-else class="shrink-0 w-16 h-16 bg-gray-800 rounded flex items-center justify-center">
            <span class="text-xs font-bold text-gray-500">{{ fileExt }}</span>
          </div>
          <div class="min-w-0">
            <p class="text-sm font-medium truncate">{{ uploadedFile!.name }}</p>
            <p class="text-xs text-gray-500">{{ fileSize }}</p>
          </div>
        </div>

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
