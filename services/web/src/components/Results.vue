<script setup lang="ts">
import { ref, computed } from 'vue'
import type { VerifyOutput } from '../types'
import ProofStatus from './ProofStatus.vue'
import SubmitPanel from './SubmitPanel.vue'

const props = defineProps<{
  result: VerifyOutput
  file: File
}>()

const trustColor = computed(() => {
  switch (props.result.trust_list_match) {
    case 'official': return 'text-green-400 bg-green-950'
    case 'curated': return 'text-yellow-400 bg-yellow-950'
    case 'untrusted': return 'text-red-400 bg-red-950'
    default: return 'text-gray-400 bg-gray-800'
  }
})

const trustLabel = computed(() => {
  if (!props.result.has_c2pa) return 'No C2PA'
  return props.result.trust_list_match || 'unknown'
})

const proofHex = ref<string | null>(null)
const publicInputsHex = ref<string | null>(null)

function onProved(proof: string, publicInputs: string) {
  proofHex.value = proof
  publicInputsHex.value = publicInputs
}

const fields = computed(() => {
  const r = props.result
  return [
    { label: 'Content Hash', value: r.content_hash },
    { label: 'Validation State', value: r.validation_state },
    { label: 'Digital Source Type', value: r.digital_source_type },
    { label: 'Issuer', value: r.issuer },
    { label: 'Common Name', value: r.common_name },
    { label: 'Software Agent', value: r.software_agent },
    { label: 'Signing Time', value: r.signing_time },
    { label: 'Format', value: r.format },
  ].filter(f => f.value)
})
</script>

<template>
  <div class="space-y-6">
    <!-- Trust badge -->
    <div class="flex items-center gap-3">
      <span :class="['px-3 py-1 rounded-full text-sm font-medium', trustColor]">
        {{ trustLabel }}
      </span>
      <span class="text-gray-400 text-sm">{{ result.has_c2pa ? 'C2PA metadata found' : 'No C2PA metadata' }}</span>
    </div>

    <!-- Fields -->
    <div class="bg-gray-900 rounded-lg border border-gray-800 divide-y divide-gray-800">
      <div v-for="field in fields" :key="field.label" class="flex px-4 py-3">
        <span class="text-gray-400 w-44 shrink-0 text-sm">{{ field.label }}</span>
        <span class="text-gray-100 text-sm break-all">{{ field.value }}</span>
      </div>
    </div>

    <!-- Actions -->
    <div v-if="result.has_c2pa && result.content_hash" class="space-y-4">
      <ProofStatus :file="file" @proved="onProved" />
      <SubmitPanel
        :result="result"
        :proof="proofHex"
        :public-inputs="publicInputsHex"
      />
    </div>
  </div>
</template>
