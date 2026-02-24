<script setup lang="ts">
</script>

<template>
  <div class="space-y-6">
    <h1 class="text-2xl font-bold">ZK Prove</h1>
    <p class="text-gray-400 max-w-2xl">
      Generate a zero-knowledge Groth16 proof that C2PA verification was performed correctly,
      then submit the proof on-chain for trustless verification. No trusted verifier required.
    </p>

    <!-- Coming soon overlay -->
    <div class="relative">
      <div class="absolute inset-0 bg-gray-950/70 backdrop-blur-sm z-10 rounded-xl flex items-center justify-center">
        <div class="text-center space-y-2">
          <p class="text-sm text-gray-500 max-w-xs">ZK proof generation requires a GPU-enabled server. This feature will be available when the GPU prover is deployed.</p>
        </div>
      </div>

      <div class="space-y-4 pointer-events-none select-none opacity-40">
        <!-- Upload area (disabled) -->
        <div class="border-2 border-dashed border-gray-700 rounded-xl p-12 text-center">
          <p class="text-lg mb-1">Drop a media file here</p>
          <p class="text-sm text-gray-500">or click to browse</p>
        </div>

        <!-- Pipeline preview -->
        <div class="space-y-3">
          <h3 class="text-sm font-medium text-gray-400 uppercase tracking-wide">Pipeline</h3>

          <div class="bg-gray-900 rounded-lg border border-gray-800 p-4">
            <div class="flex items-center gap-3">
              <span class="shrink-0 w-6 h-6 rounded-full bg-gray-800 text-gray-400 text-xs font-bold flex items-center justify-center">1</span>
              <div>
                <span class="text-sm font-medium text-gray-400">Verify C2PA</span>
                <p class="text-xs text-gray-600">Check provenance metadata and signatures</p>
              </div>
            </div>
          </div>

          <div class="bg-gray-900 rounded-lg border border-gray-800 p-4">
            <div class="flex items-center gap-3">
              <span class="shrink-0 w-6 h-6 rounded-full bg-gray-800 text-gray-400 text-xs font-bold flex items-center justify-center">2</span>
              <div>
                <span class="text-sm font-medium text-gray-400">Generate ZK Proof</span>
                <p class="text-xs text-gray-600">SP1 zkVM Groth16 proof of verification (requires GPU)</p>
              </div>
            </div>
          </div>

          <div class="bg-gray-900 rounded-lg border border-gray-800 p-4">
            <div class="flex items-center gap-3">
              <span class="shrink-0 w-6 h-6 rounded-full bg-gray-800 text-gray-400 text-xs font-bold flex items-center justify-center">3</span>
              <div>
                <span class="text-sm font-medium text-gray-400">Submit to Solana</span>
                <p class="text-xs text-gray-600">On-chain Groth16 verification — fully trustless</p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
