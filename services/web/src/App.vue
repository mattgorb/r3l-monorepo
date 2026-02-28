<script setup lang="ts">
import { useRoute } from 'vue-router'

const route = useRoute()
const isActive = (path: string) => route.path === path || route.path.startsWith(path + '/')
</script>

<template>
  <div class="min-h-screen bg-gray-950 text-gray-100">
    <nav class="border-b border-gray-800">
      <div class="max-w-5xl mx-auto px-6 py-3 flex items-center gap-8">
        <router-link to="/" class="text-lg font-bold tracking-tight hover:text-white">R3L</router-link>
        <div class="flex gap-6">
          <router-link
            to="/attest"
            :class="['text-sm transition-colors', isActive('/attest') ? 'text-white font-medium' : 'text-gray-400 hover:text-white']"
          >Attest</router-link>
          <router-link
            to="/search"
            :class="['text-sm transition-colors', isActive('/search') ? 'text-white font-medium' : 'text-gray-400 hover:text-white']"
          >Search</router-link>
          <router-link
            to="/prove"
            :class="['text-sm transition-colors', isActive('/prove') ? 'text-white font-medium' : 'text-gray-400 hover:text-white']"
          >ZK Prove</router-link>
          <router-link
            to="/docs"
            :class="['text-sm transition-colors', isActive('/docs') ? 'text-white font-medium' : 'text-gray-400 hover:text-white']"
          >Docs</router-link>
          <router-link
            to="/account"
            :class="['text-sm transition-colors', isActive('/account') ? 'text-white font-medium' : 'text-gray-400 hover:text-white']"
          >Account</router-link>
        </div>
      </div>
    </nav>
    <div class="max-w-5xl mx-auto px-6 py-12">
      <router-view />
    </div>
  </div>
</template>
