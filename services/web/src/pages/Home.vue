<template>
  <div class="space-y-12">
    <!-- Hero -->
    <div class="space-y-4">
      <h1 class="text-4xl font-bold">Provenance Attestation</h1>
      <p class="text-lg text-gray-400 max-w-2xl">
        Verify media provenance, bind creator identity via wallet signature, and store permanent attestations on Solana.
      </p>
      <div class="flex gap-4 pt-2">
        <router-link to="/verify" class="px-5 py-2.5 bg-blue-600 hover:bg-blue-500 rounded-lg text-sm font-medium transition-colors">
          Verify a File
        </router-link>
        <router-link to="/search" class="px-5 py-2.5 bg-gray-800 hover:bg-gray-700 rounded-lg text-sm font-medium transition-colors">
          Search
        </router-link>
      </div>
    </div>

    <!-- Trust Signals -->
    <div class="space-y-4">
      <h2 class="text-xl font-semibold">Trust Signals</h2>
      <p class="text-sm text-gray-400">Every attestation is anchored on-chain. Files can carry multiple independent trust signals.</p>

      <!-- Primary: Wallet -->
      <div class="bg-gray-900 rounded-lg border border-purple-700/60 p-5 space-y-2">
        <div class="flex items-center gap-2">
          <span class="text-purple-400 text-lg">&#9919;</span>
          <p class="text-base font-semibold text-purple-400">Wallet Identity</p>
        </div>
        <p class="text-sm text-gray-400">
          Creator signs a message with their Solana wallet, verified on-chain via Ed25519 precompile.
          The signature is stored in the attestation PDA — anyone can independently verify it.
          This is the primary trust anchor for all content, with or without C2PA.
        </p>
      </div>

      <!-- Secondary: C2PA -->
      <div class="bg-gray-900 rounded-lg border border-green-800/50 p-5 space-y-2">
        <div class="flex items-center gap-2">
          <span class="text-green-400 text-lg">&#10003;</span>
          <p class="text-base font-semibold text-green-400">C2PA Provenance</p>
        </div>
        <p class="text-sm text-gray-400">
          If the file contains C2PA metadata, R3L extracts and validates it — issuer, software agent, signing time,
          digital source type, and trust list status (official, curated, or untrusted signer).
          Strongest hardware/software provenance when available.
        </p>
      </div>
    </div>

    <!-- How it works -->
    <div class="space-y-4">
      <h2 class="text-xl font-semibold">How It Works</h2>
      <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
        <div class="bg-gray-900 rounded-lg border border-gray-800 p-5 space-y-2">
          <p class="text-sm font-semibold text-gray-200">1. Upload</p>
          <p class="text-xs text-gray-500">Upload any media file — image, video, audio, or PDF. R3L computes the SHA-256 content hash.</p>
        </div>
        <div class="bg-gray-900 rounded-lg border border-gray-800 p-5 space-y-2">
          <p class="text-sm font-semibold text-gray-200">2. Verify + Sign</p>
          <p class="text-xs text-gray-500">R3L checks for C2PA provenance metadata. You sign the content hash with your wallet to bind your identity.</p>
        </div>
        <div class="bg-gray-900 rounded-lg border border-gray-800 p-5 space-y-2">
          <p class="text-sm font-semibold text-gray-200">3. Attest On-Chain</p>
          <p class="text-xs text-gray-500">Verification results, wallet signature, and metadata are written to a Solana PDA. Anyone can look it up by content hash.</p>
        </div>
        <div class="bg-gray-900 rounded-lg border border-blue-800/50 p-5 space-y-2">
          <p class="text-sm font-semibold text-blue-400">4. Query</p>
          <p class="text-xs text-gray-500">AI platforms, apps, and integrations query R3L's API for structured trust verdicts — trusted, attested, or unknown.</p>
        </div>
      </div>
    </div>

    <!-- Optional: Email -->
    <div class="space-y-3">
      <h2 class="text-sm font-semibold text-gray-500 uppercase tracking-wide">Optional Add-on</h2>
      <div class="bg-gray-900/50 rounded-lg border border-gray-800/50 p-4">
        <div class="flex items-center gap-2">
          <span class="text-yellow-500 text-sm">&#9993;</span>
          <p class="text-sm font-medium text-yellow-500/80">Email Domain Verification</p>
        </div>
        <p class="text-xs text-gray-600 mt-1">
          Verify ownership of an email domain (e.g. @nytimes.com) and link it to the attestation.
          Adds an organizational identity layer on top of wallet and C2PA signals. Coming soon.
        </p>
      </div>
    </div>
  </div>
</template>
