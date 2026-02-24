<script setup lang="ts">
// Email identity verification disabled for v1.
// This page redirects to /verify via the router.
</script>

<template>
  <div class="text-center py-12">
    <p class="text-gray-400">Email identity verification is not available yet.</p>
    <router-link to="/verify" class="text-blue-400 hover:text-blue-300 text-sm mt-2 inline-block">
      Go to Verify & Attest
    </router-link>
  </div>
</template>
