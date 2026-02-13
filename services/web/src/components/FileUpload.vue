<script setup lang="ts">
import { ref } from 'vue'
import type { VerifyOutput } from '../types'
import { verifyFile } from '../api'

const emit = defineEmits<{
  verified: [result: VerifyOutput, file: File]
}>()

const loading = ref(false)
const error = ref<string | null>(null)
const dragging = ref(false)

async function handleFile(file: File) {
  loading.value = true
  error.value = null
  try {
    const result = await verifyFile(file)
    emit('verified', result, file)
  } catch (e: any) {
    error.value = e.response?.data || e.message
  } finally {
    loading.value = false
  }
}

function onDrop(e: DragEvent) {
  dragging.value = false
  const file = e.dataTransfer?.files[0]
  if (file) handleFile(file)
}

function onInput(e: Event) {
  const file = (e.target as HTMLInputElement).files?.[0]
  if (file) handleFile(file)
}
</script>

<template>
  <div
    @dragover.prevent="dragging = true"
    @dragleave="dragging = false"
    @drop.prevent="onDrop"
    :class="[
      'border-2 border-dashed rounded-xl p-12 text-center transition-colors cursor-pointer',
      dragging ? 'border-blue-400 bg-blue-950/30' : 'border-gray-700 hover:border-gray-500',
    ]"
    @click="($refs.input as HTMLInputElement).click()"
  >
    <input
      ref="input"
      type="file"
      class="hidden"
      accept="image/*,video/*,audio/*"
      @change="onInput"
    />

    <div v-if="loading" class="text-gray-400">
      <svg class="animate-spin h-8 w-8 mx-auto mb-3 text-blue-400" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
      </svg>
      Verifying...
    </div>
    <div v-else>
      <p class="text-lg mb-1">Drop a media file here</p>
      <p class="text-sm text-gray-500">or click to browse</p>
    </div>
  </div>

  <p v-if="error" class="mt-3 text-red-400 text-sm">{{ error }}</p>
</template>
