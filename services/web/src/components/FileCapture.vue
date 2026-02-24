<script setup lang="ts">
import { ref } from 'vue'

const emit = defineEmits<{
  captured: [file: File]
}>()

const dragging = ref(false)

function onDrop(e: DragEvent) {
  dragging.value = false
  const file = e.dataTransfer?.files[0]
  if (file) emit('captured', file)
}

function onInput(e: Event) {
  const file = (e.target as HTMLInputElement).files?.[0]
  if (file) emit('captured', file)
}
</script>

<template>
  <div
    @dragover.prevent="dragging = true"
    @dragleave="dragging = false"
    @drop.prevent="onDrop"
    :class="[
      'border-2 border-dashed rounded-xl p-12 text-center transition-colors cursor-pointer',
      dragging ? 'border-yellow-400 bg-yellow-950/30' : 'border-gray-700 hover:border-gray-500',
    ]"
    @click="($refs.input as HTMLInputElement).click()"
  >
    <input
      ref="input"
      type="file"
      class="hidden"
      accept="image/*,video/*,audio/*,.pdf,.mp4,.mov,.heif,.heic,.txt,.csv,.md,.html,text/plain,text/csv,text/markdown,text/html"
      @change="onInput"
    />
    <p class="text-lg mb-1">Drop a media file here</p>
    <p class="text-sm text-gray-500">or click to browse</p>
  </div>
</template>
