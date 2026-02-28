<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import {
  authEmailStart, authEmailVerify, authWalletChallenge, authWalletVerify, getMe,
  linkEmailStart, linkEmailVerify, linkWallet, updatePrivacyMode,
  registerOrg, verifyOrgDns, verifyOrgEmail, resendOrgCode,
  createOrgKey, listOrgKeys, revokeOrgKey,
} from '../api'
import type { MeResponse, OrgKeyItem } from '../types'

// ── Core state ──────────────────────────────────────────────────────
const apiKey = ref(localStorage.getItem('r3l_api_key') || '')
const userInfo = ref<MeResponse | null>(null)
const error = ref('')
const loading = ref(false)
const copied = ref('')
const authTab = ref<'email' | 'wallet' | 'org'>('email')

// Shown once after successful auth
const newApiKey = ref('')

const isAuthenticated = computed(() => !!userInfo.value)
const authType = computed(() => userInfo.value?.type)

// ── Email auth state ────────────────────────────────────────────────
const emailAddr = ref('')
const emailPending = ref(false)
const emailDevCode = ref('')
const emailCode = ref('')
const emailError = ref('')
const emailLocked = ref(false)

// ── Wallet auth state ───────────────────────────────────────────────
type WalletMode = 'choose' | 'phantom' | 'manual'
const walletMode = ref<WalletMode>('choose')
const walletConnected = ref(false)
const walletPubkey = ref<string | null>(null)
const walletLoading = ref(false)
const walletError = ref('')
const walletChallenge = ref('')
const manualPubkey = ref('')
const manualSignature = ref('')
const hasPhantom = computed(() => !!(window as any).solana?.isPhantom)

// ── Org auth state ──────────────────────────────────────────────────
const regDomain = ref('')
const regMethod = ref<'dns' | 'email'>('dns')
const regEmail = ref('')
const regName = ref('')
const pendingVerify = ref<any>(null)
const orgEmailCode = ref('')
const orgVerifyError = ref('')
const orgResending = ref(false)
const orgResendSuccess = ref(false)
const orgLocked = ref(false)

// ── Org dashboard state ─────────────────────────────────────────────
const orgKeys = ref<OrgKeyItem[]>([])
const newKeyEmail = ref('')
const newKeyRole = ref<'attester' | 'admin'>('attester')
const createdKey = ref('')

// ── Core functions ──────────────────────────────────────────────────

async function loadUser(silent = false) {
  if (!apiKey.value) return
  loading.value = true
  if (!silent) error.value = ''
  try {
    userInfo.value = await getMe(apiKey.value)
    localStorage.setItem('r3l_api_key', apiKey.value)
    if (authType.value === 'org') await loadOrgKeys()
  } catch {
    if (silent) {
      apiKey.value = ''
      localStorage.removeItem('r3l_api_key')
    } else {
      error.value = 'Invalid API key'
    }
    userInfo.value = null
  } finally {
    loading.value = false
  }
}

function logout() {
  apiKey.value = ''
  userInfo.value = null
  orgKeys.value = []
  newApiKey.value = ''
  localStorage.removeItem('r3l_api_key')
}

function copyText(text: string, label: string) {
  navigator.clipboard.writeText(text)
  copied.value = label
  setTimeout(() => copied.value = '', 2000)
}

function tabClass(tab: string) {
  return [
    'px-4 py-2.5 text-sm transition-colors cursor-pointer',
    authTab.value === tab ? 'text-white border-b-2 border-purple-500' : 'text-gray-500 hover:text-gray-300',
  ]
}

// ── Email auth ──────────────────────────────────────────────────────

async function handleEmailStart() {
  emailError.value = ''
  emailLocked.value = false
  loading.value = true
  try {
    const resp = await authEmailStart(emailAddr.value)
    emailPending.value = true
    emailDevCode.value = resp.dev_code || ''
    emailCode.value = ''
  } catch (e: any) {
    emailError.value = e.response?.data?.detail || 'Failed to send code'
  } finally {
    loading.value = false
  }
}

async function handleEmailVerify() {
  emailError.value = ''
  loading.value = true
  try {
    const resp = await authEmailVerify(emailAddr.value, emailCode.value)
    apiKey.value = resp.api_key
    newApiKey.value = resp.api_key
    emailPending.value = false
    await loadUser()
  } catch (e: any) {
    const status = e.response?.status
    const detail = e.response?.data?.detail || 'Verification failed'
    emailError.value = detail
    emailCode.value = ''
    if (status === 429 || status === 410) emailLocked.value = true
  } finally {
    loading.value = false
  }
}

function cancelEmail() {
  emailPending.value = false
  emailError.value = ''
  emailCode.value = ''
  emailDevCode.value = ''
  emailLocked.value = false
}

// ── Wallet auth ─────────────────────────────────────────────────────

async function connectPhantom() {
  walletError.value = ''
  const provider = (window as any).solana
  if (!provider?.isPhantom) {
    walletError.value = 'Phantom wallet not found.'
    return
  }
  try {
    const resp = await provider.connect()
    walletPubkey.value = resp.publicKey.toString()
    walletConnected.value = true
    // Fetch challenge immediately
    await fetchChallenge()
  } catch (e: any) {
    walletError.value = e.message || 'Failed to connect wallet'
  }
}

async function fetchChallenge() {
  try {
    const resp = await authWalletChallenge()
    walletChallenge.value = resp.challenge
  } catch (e: any) {
    walletError.value = 'Failed to get challenge'
  }
}

async function signAndVerifyPhantom() {
  if (!walletPubkey.value || !walletChallenge.value) return
  walletLoading.value = true
  walletError.value = ''

  const provider = (window as any).solana
  try {
    const encoded = new TextEncoder().encode(walletChallenge.value)
    const { signature } = await provider.signMessage(encoded, 'utf8')
    const bs58 = await import('bs58')
    const sigBase58 = bs58.default.encode(signature)

    const resp = await authWalletVerify({
      pubkey: walletPubkey.value,
      message: walletChallenge.value,
      signature: sigBase58,
    })
    apiKey.value = resp.api_key
    newApiKey.value = resp.api_key
    await loadUser()
  } catch (e: any) {
    const detail = e.response?.data?.detail || e.message || 'Wallet verification failed'
    walletError.value = detail
  } finally {
    walletLoading.value = false
  }
}

async function handleManualWallet() {
  if (!manualPubkey.value || !manualSignature.value || !walletChallenge.value) return
  walletLoading.value = true
  walletError.value = ''
  try {
    const resp = await authWalletVerify({
      pubkey: manualPubkey.value.trim(),
      message: walletChallenge.value,
      signature: manualSignature.value.trim(),
    })
    apiKey.value = resp.api_key
    newApiKey.value = resp.api_key
    await loadUser()
  } catch (e: any) {
    const detail = e.response?.data?.detail || 'Wallet verification failed'
    walletError.value = detail
  } finally {
    walletLoading.value = false
  }
}

// ── Org auth ────────────────────────────────────────────────────────

async function handleOrgRegister() {
  error.value = ''
  orgVerifyError.value = ''
  orgLocked.value = false
  loading.value = true
  try {
    const resp = await registerOrg({
      domain: regDomain.value,
      method: regMethod.value,
      admin_email: regEmail.value || undefined,
      name: regName.value || undefined,
    })
    pendingVerify.value = resp
    orgEmailCode.value = ''
  } catch (e: any) {
    const detail = e.response?.data?.detail || 'Registration failed'
    // If backend says domain is verified and needs email, switch to email method
    if (e.response?.status === 400 && detail.includes('email verification to log in')) {
      regMethod.value = 'email'
      error.value = 'This domain is already verified. Enter an admin email to log in.'
    } else {
      error.value = detail
    }
  } finally {
    loading.value = false
  }
}

async function handleOrgVerifyDns() {
  orgVerifyError.value = ''
  loading.value = true
  try {
    const resp = await verifyOrgDns(pendingVerify.value.domain)
    apiKey.value = resp.api_key
    newApiKey.value = resp.api_key
    pendingVerify.value = null
    await loadUser()
  } catch (e: any) {
    orgVerifyError.value = e.response?.data?.detail || 'DNS verification failed'
  } finally {
    loading.value = false
  }
}

async function handleOrgVerifyEmail() {
  orgVerifyError.value = ''
  loading.value = true
  try {
    const resp = await verifyOrgEmail(regEmail.value || pendingVerify.value.email, orgEmailCode.value)
    apiKey.value = resp.api_key
    newApiKey.value = resp.api_key
    pendingVerify.value = null
    orgLocked.value = false
    await loadUser()
  } catch (e: any) {
    const status = e.response?.status
    orgVerifyError.value = e.response?.data?.detail || 'Verification failed'
    orgEmailCode.value = ''
    if (status === 429 || status === 410) orgLocked.value = true
  } finally {
    loading.value = false
  }
}

async function handleOrgResend() {
  orgVerifyError.value = ''
  orgResending.value = true
  orgResendSuccess.value = false
  orgLocked.value = false
  orgEmailCode.value = ''
  try {
    const email = regEmail.value || pendingVerify.value?.email
    const domain = pendingVerify.value?.domain || regDomain.value
    const resp = await resendOrgCode(domain, email)
    pendingVerify.value = { ...pendingVerify.value, ...resp }
    orgResendSuccess.value = true
    setTimeout(() => orgResendSuccess.value = false, 4000)
  } catch (e: any) {
    orgVerifyError.value = e.response?.data?.detail || 'Failed to resend code'
  } finally {
    orgResending.value = false
  }
}

function cancelOrg() {
  pendingVerify.value = null
  orgVerifyError.value = ''
  orgEmailCode.value = ''
  orgLocked.value = false
}

// ── Org dashboard ───────────────────────────────────────────────────

async function loadOrgKeys() {
  if (!apiKey.value) return
  try {
    const resp = await listOrgKeys(apiKey.value)
    orgKeys.value = resp.keys
  } catch {}
}

async function handleCreateKey() {
  error.value = ''
  createdKey.value = ''
  try {
    const resp = await createOrgKey(apiKey.value, {
      email: newKeyEmail.value || undefined,
      role: newKeyRole.value,
    })
    createdKey.value = resp.api_key
    newKeyEmail.value = ''
    await loadOrgKeys()
  } catch (e: any) {
    error.value = e.response?.data?.detail || 'Failed to create key'
  }
}

async function handleRevoke(keyId: number) {
  try {
    await revokeOrgKey(apiKey.value, keyId)
    await loadOrgKeys()
  } catch (e: any) {
    error.value = e.response?.data?.detail || 'Failed to revoke key'
  }
}

// ── Privacy mode ────────────────────────────────────────────────────

const privacyLoading = ref(false)

async function togglePrivacyMode() {
  if (!userInfo.value || userInfo.value.type !== 'individual') return
  privacyLoading.value = true
  try {
    const newValue = !userInfo.value.privacy_mode
    const resp = await updatePrivacyMode(apiKey.value, newValue)
    userInfo.value = { ...userInfo.value, privacy_mode: resp.privacy_mode }
  } catch (e: any) {
    error.value = e.response?.data?.detail || 'Failed to update privacy mode'
  } finally {
    privacyLoading.value = false
  }
}

// ── Identity linking ────────────────────────────────────────────────

type LinkMode = 'none' | 'email' | 'wallet'
const linkMode = ref<LinkMode>('none')
const linkError = ref('')

// Link email state
const linkEmailAddr = ref('')
const linkEmailPending = ref(false)
const linkEmailDevCode = ref('')
const linkEmailCode = ref('')
const linkEmailLocked = ref(false)

// Link wallet state
type LinkWalletMode = 'choose' | 'phantom' | 'manual'
const linkWalletMode = ref<LinkWalletMode>('choose')
const linkWalletConnected = ref(false)
const linkWalletPubkey = ref<string | null>(null)
const linkWalletLoading = ref(false)
const linkWalletChallenge = ref('')
const linkManualPubkey = ref('')
const linkManualSignature = ref('')

function cancelLink() {
  linkMode.value = 'none'
  linkError.value = ''
  linkEmailAddr.value = ''
  linkEmailPending.value = false
  linkEmailDevCode.value = ''
  linkEmailCode.value = ''
  linkEmailLocked.value = false
  linkWalletMode.value = 'choose'
  linkWalletConnected.value = false
  linkWalletPubkey.value = null
  linkWalletLoading.value = false
  linkWalletChallenge.value = ''
  linkManualPubkey.value = ''
  linkManualSignature.value = ''
}

async function handleLinkEmailStart() {
  linkError.value = ''
  linkEmailLocked.value = false
  loading.value = true
  try {
    const resp = await linkEmailStart(apiKey.value, linkEmailAddr.value)
    linkEmailPending.value = true
    linkEmailDevCode.value = resp.dev_code || ''
    linkEmailCode.value = ''
  } catch (e: any) {
    linkError.value = e.response?.data?.detail || 'Failed to send code'
  } finally {
    loading.value = false
  }
}

async function handleLinkEmailVerify() {
  linkError.value = ''
  loading.value = true
  try {
    await linkEmailVerify(apiKey.value, linkEmailAddr.value, linkEmailCode.value)
    cancelLink()
    await loadUser()
  } catch (e: any) {
    const status = e.response?.status
    linkError.value = e.response?.data?.detail || 'Verification failed'
    linkEmailCode.value = ''
    if (status === 429 || status === 410) linkEmailLocked.value = true
  } finally {
    loading.value = false
  }
}

async function linkConnectPhantom() {
  linkError.value = ''
  const provider = (window as any).solana
  if (!provider?.isPhantom) {
    linkError.value = 'Phantom wallet not found.'
    return
  }
  try {
    const resp = await provider.connect()
    linkWalletPubkey.value = resp.publicKey.toString()
    linkWalletConnected.value = true
    await linkFetchChallenge()
  } catch (e: any) {
    linkError.value = e.message || 'Failed to connect wallet'
  }
}

async function linkFetchChallenge() {
  try {
    const resp = await authWalletChallenge()
    linkWalletChallenge.value = resp.challenge
  } catch {
    linkError.value = 'Failed to get challenge'
  }
}

async function linkSignAndVerifyPhantom() {
  if (!linkWalletPubkey.value || !linkWalletChallenge.value) return
  linkWalletLoading.value = true
  linkError.value = ''

  const provider = (window as any).solana
  try {
    const encoded = new TextEncoder().encode(linkWalletChallenge.value)
    const { signature } = await provider.signMessage(encoded, 'utf8')
    const bs58 = await import('bs58')
    const sigBase58 = bs58.default.encode(signature)

    await linkWallet(apiKey.value, {
      pubkey: linkWalletPubkey.value,
      message: linkWalletChallenge.value,
      signature: sigBase58,
    })
    cancelLink()
    await loadUser()
  } catch (e: any) {
    linkError.value = e.response?.data?.detail || e.message || 'Wallet linking failed'
  } finally {
    linkWalletLoading.value = false
  }
}

async function handleLinkManualWallet() {
  if (!linkManualPubkey.value || !linkManualSignature.value || !linkWalletChallenge.value) return
  linkWalletLoading.value = true
  linkError.value = ''
  try {
    await linkWallet(apiKey.value, {
      pubkey: linkManualPubkey.value.trim(),
      message: linkWalletChallenge.value,
      signature: linkManualSignature.value.trim(),
    })
    cancelLink()
    await loadUser()
  } catch (e: any) {
    linkError.value = e.response?.data?.detail || 'Wallet linking failed'
  } finally {
    linkWalletLoading.value = false
  }
}

// ── Lifecycle ───────────────────────────────────────────────────────

onMounted(() => {
  // Migrate old localStorage key
  const oldKey = localStorage.getItem('r3l_org_key')
  if (oldKey && !apiKey.value) {
    apiKey.value = oldKey
    localStorage.setItem('r3l_api_key', oldKey)
    localStorage.removeItem('r3l_org_key')
  }
  if (apiKey.value) loadUser(true)
})
</script>

<template>
  <div class="space-y-8">
    <div>
      <h1 class="text-2xl font-bold">Account</h1>
      <p class="text-sm text-gray-500 mt-1">Sign in with email, wallet, or register your organization.</p>
    </div>

    <!-- Top-level error -->
    <div v-if="error" class="bg-red-900/30 border border-red-800 rounded-lg px-4 py-3 text-sm text-red-300">
      {{ error }}
    </div>

    <!-- ═══════════════════ AUTHENTICATED ═══════════════════ -->
    <template v-if="isAuthenticated">

      <!-- New API key banner -->
      <div v-if="newApiKey" class="bg-green-900/20 border border-green-800 rounded-lg px-5 py-4 space-y-2">
        <p class="text-sm font-semibold text-green-400">You're in! Here's your API key:</p>
        <p class="text-xs text-green-300/70">Save this now — you'll need it to make API calls.</p>
        <div class="flex items-center gap-2">
          <code class="text-sm font-mono text-green-300 bg-gray-950 px-3 py-1.5 rounded flex-1 break-all">{{ newApiKey }}</code>
          <button @click="copyText(newApiKey, 'newkey')" class="text-xs bg-green-800 text-green-200 px-3 py-1.5 rounded hover:bg-green-700 transition-colors cursor-pointer shrink-0">
            {{ copied === 'newkey' ? 'Copied' : 'Copy' }}
          </button>
        </div>
        <button @click="newApiKey = ''" class="text-xs text-gray-500 hover:text-gray-300 transition-colors cursor-pointer mt-1">Dismiss</button>
      </div>

      <!-- ── Individual dashboard ── -->
      <template v-if="authType === 'individual'">
        <div class="bg-gray-900 rounded-lg border border-gray-800 p-6 space-y-4">
          <div class="flex items-center justify-between">
            <h2 class="text-lg font-semibold">{{ userInfo!.name }}</h2>
            <button @click="logout" class="text-xs text-gray-500 hover:text-gray-300 transition-colors cursor-pointer">Logout</button>
          </div>

          <!-- Linked identities -->
          <div class="space-y-2">
            <!-- Email -->
            <div class="flex items-center gap-3 bg-gray-950 rounded-lg px-4 py-2.5 border border-gray-800">
              <span class="text-xs text-gray-500 w-14 shrink-0">Email</span>
              <template v-if="userInfo!.email">
                <span class="text-sm text-gray-300 flex-1">{{ userInfo!.email }}</span>
                <span class="text-xs text-green-500">linked</span>
              </template>
              <template v-else>
                <span class="text-sm text-gray-600 flex-1">not linked</span>
                <button @click="linkMode = 'email'" class="text-xs text-purple-400 hover:text-purple-300 transition-colors cursor-pointer">Link Email</button>
              </template>
            </div>
            <!-- Wallet -->
            <div class="flex items-center gap-3 bg-gray-950 rounded-lg px-4 py-2.5 border border-gray-800">
              <span class="text-xs text-gray-500 w-14 shrink-0">Wallet</span>
              <template v-if="userInfo!.wallet_pubkey">
                <span class="text-sm text-gray-300 font-mono flex-1">{{ userInfo!.wallet_pubkey!.slice(0, 4) }}...{{ userInfo!.wallet_pubkey!.slice(-4) }}</span>
                <span class="text-xs text-green-500">linked</span>
              </template>
              <template v-else>
                <span class="text-sm text-gray-600 flex-1">not linked</span>
                <button @click="linkMode = 'wallet'" class="text-xs text-purple-400 hover:text-purple-300 transition-colors cursor-pointer">Link Wallet</button>
              </template>
            </div>
          </div>

          <p class="text-xs text-gray-600">Authenticated via {{ userInfo!.auth_method }}</p>

          <!-- Privacy mode toggle -->
          <div class="flex items-center justify-between bg-gray-950 rounded-lg px-4 py-3 border border-gray-800">
            <div>
              <p class="text-sm text-gray-300">Privacy Mode</p>
              <p class="text-xs text-gray-600 mt-0.5">Keep your identity off the public Solana ledger.</p>
            </div>
            <button
              @click="togglePrivacyMode"
              :disabled="privacyLoading"
              :class="[
                'relative inline-flex h-6 w-11 items-center rounded-full transition-colors cursor-pointer',
                userInfo!.privacy_mode ? 'bg-purple-600' : 'bg-gray-700',
                privacyLoading ? 'opacity-50' : '',
              ]"
            >
              <span
                :class="[
                  'inline-block h-4 w-4 transform rounded-full bg-white transition-transform',
                  userInfo!.privacy_mode ? 'translate-x-6' : 'translate-x-1',
                ]"
              />
            </button>
          </div>
        </div>

        <!-- ── Link email flow ── -->
        <div v-if="linkMode === 'email'" class="bg-gray-900 rounded-lg border border-purple-800/40 p-6 space-y-4">
          <h3 class="text-sm font-semibold text-purple-400">Link Email</h3>
          <div v-if="linkError" class="bg-red-900/30 border border-red-800 rounded-lg px-4 py-2.5 text-xs text-red-300">{{ linkError }}</div>

          <template v-if="!linkEmailPending">
            <div>
              <label class="text-xs text-gray-500 block mb-1">Email address</label>
              <input v-model="linkEmailAddr" placeholder="you@example.com" class="w-full bg-gray-950 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-gray-500" @keyup.enter="handleLinkEmailStart()" />
            </div>
            <div class="flex items-center gap-3">
              <button @click="handleLinkEmailStart" :disabled="loading || !linkEmailAddr" class="bg-purple-700 hover:bg-purple-600 text-white text-sm px-5 py-2 rounded-lg transition-colors disabled:opacity-50 cursor-pointer">
                {{ loading ? 'Sending...' : 'Send Code' }}
              </button>
              <button @click="cancelLink" class="text-sm text-gray-500 hover:text-gray-300 transition-colors cursor-pointer">Cancel</button>
            </div>
          </template>

          <template v-else>
            <p class="text-sm text-gray-400">Enter the 6-digit code sent to <strong class="text-gray-200">{{ linkEmailAddr }}</strong></p>
            <div v-if="linkEmailDevCode" class="bg-yellow-900/20 border border-yellow-800 rounded-lg px-4 py-2 text-xs text-yellow-400">
              Dev mode — code: <strong>{{ linkEmailDevCode }}</strong>
            </div>
            <template v-if="!linkEmailLocked">
              <input v-model="linkEmailCode" placeholder="000000" maxlength="6" class="w-40 bg-gray-950 border border-gray-700 rounded-lg px-3 py-2 text-lg font-mono tracking-widest text-center focus:outline-none focus:border-gray-500" @keyup.enter="linkEmailCode.length === 6 && handleLinkEmailVerify()" />
              <div class="flex items-center gap-3">
                <button @click="handleLinkEmailVerify" :disabled="loading || linkEmailCode.length !== 6" class="bg-purple-700 hover:bg-purple-600 text-white text-sm px-5 py-2 rounded-lg transition-colors disabled:opacity-50 cursor-pointer">
                  {{ loading ? 'Verifying...' : 'Verify Code' }}
                </button>
                <button @click="cancelLink" class="text-sm text-gray-500 hover:text-gray-300 transition-colors cursor-pointer">Cancel</button>
              </div>
            </template>
            <template v-else>
              <p class="text-xs text-gray-500">Too many attempts. Please start over.</p>
              <button @click="cancelLink" class="text-sm text-gray-500 hover:text-gray-300 transition-colors cursor-pointer">Start over</button>
            </template>
          </template>
        </div>

        <!-- ── Link wallet flow ── -->
        <div v-if="linkMode === 'wallet'" class="bg-gray-900 rounded-lg border border-purple-800/40 p-6 space-y-4">
          <h3 class="text-sm font-semibold text-purple-400">Link Wallet</h3>
          <div v-if="linkError" class="bg-red-900/30 border border-red-800 rounded-lg px-4 py-2.5 text-xs text-red-300">{{ linkError }}</div>

          <!-- Mode chooser -->
          <template v-if="linkWalletMode === 'choose'">
            <div class="flex gap-3">
              <button v-if="hasPhantom" @click="linkWalletMode = 'phantom'; linkConnectPhantom()" class="bg-purple-700 hover:bg-purple-600 text-white text-sm px-5 py-2.5 rounded-lg transition-colors cursor-pointer">
                Connect Phantom
              </button>
              <button @click="linkWalletMode = 'manual'; linkFetchChallenge()" class="bg-gray-800 hover:bg-gray-700 text-white text-sm px-5 py-2.5 rounded-lg transition-colors cursor-pointer">
                Enter Key Manually
              </button>
            </div>
            <button @click="cancelLink" class="text-sm text-gray-500 hover:text-gray-300 transition-colors cursor-pointer">Cancel</button>
          </template>

          <!-- Phantom flow -->
          <template v-if="linkWalletMode === 'phantom' && linkWalletConnected">
            <div class="space-y-3">
              <div class="bg-gray-950 rounded-lg px-4 py-2.5 border border-gray-800">
                <span class="text-xs text-gray-500">Connected: </span>
                <span class="text-xs font-mono text-gray-300">{{ linkWalletPubkey }}</span>
              </div>
              <div v-if="linkWalletChallenge" class="bg-gray-950 rounded-lg px-4 py-2.5 border border-gray-800">
                <span class="text-xs text-gray-500">Challenge: </span>
                <span class="text-xs font-mono text-gray-300">{{ linkWalletChallenge }}</span>
              </div>
              <div class="flex items-center gap-3">
                <button @click="linkSignAndVerifyPhantom" :disabled="linkWalletLoading || !linkWalletChallenge" class="bg-purple-700 hover:bg-purple-600 text-white text-sm px-5 py-2 rounded-lg transition-colors disabled:opacity-50 cursor-pointer">
                  {{ linkWalletLoading ? 'Signing...' : 'Sign & Link' }}
                </button>
                <button @click="cancelLink" class="text-sm text-gray-500 hover:text-gray-300 transition-colors cursor-pointer">Cancel</button>
              </div>
            </div>
          </template>

          <!-- Manual flow -->
          <template v-if="linkWalletMode === 'manual'">
            <div class="space-y-3">
              <div v-if="linkWalletChallenge" class="bg-gray-950 rounded-lg px-4 py-2.5 border border-gray-800">
                <p class="text-xs text-gray-500 mb-1">Sign this message with your wallet:</p>
                <div class="flex items-center gap-2">
                  <code class="text-sm font-mono text-yellow-400 flex-1">{{ linkWalletChallenge }}</code>
                  <button @click="copyText(linkWalletChallenge, 'linkchallenge')" class="text-xs text-gray-600 hover:text-gray-300 transition-colors shrink-0 cursor-pointer">
                    {{ copied === 'linkchallenge' ? 'Copied' : 'Copy' }}
                  </button>
                </div>
              </div>
              <div>
                <label class="text-xs text-gray-500 block mb-1">Public key (base58)</label>
                <input v-model="linkManualPubkey" placeholder="Base58 public key" class="w-full bg-gray-950 border border-gray-700 rounded-lg px-3 py-2 text-sm font-mono focus:outline-none focus:border-gray-500" />
              </div>
              <div>
                <label class="text-xs text-gray-500 block mb-1">Signature (base58)</label>
                <input v-model="linkManualSignature" placeholder="Base58 signature" class="w-full bg-gray-950 border border-gray-700 rounded-lg px-3 py-2 text-sm font-mono focus:outline-none focus:border-gray-500" />
              </div>
              <div class="flex items-center gap-3">
                <button @click="handleLinkManualWallet" :disabled="linkWalletLoading || !linkManualPubkey || !linkManualSignature" class="bg-purple-700 hover:bg-purple-600 text-white text-sm px-5 py-2 rounded-lg transition-colors disabled:opacity-50 cursor-pointer">
                  {{ linkWalletLoading ? 'Linking...' : 'Link Wallet' }}
                </button>
                <button @click="cancelLink" class="text-sm text-gray-500 hover:text-gray-300 transition-colors cursor-pointer">Cancel</button>
              </div>
            </div>
          </template>
        </div>
      </template>

      <!-- ── Org dashboard ── -->
      <template v-if="authType === 'org'">
        <!-- Org info card -->
        <div class="bg-gray-900 rounded-lg border border-gray-800 p-6 space-y-4">
          <div class="flex items-center justify-between">
            <div>
              <h2 class="text-lg font-semibold">{{ userInfo!.org!.name || userInfo!.org!.domain }}</h2>
              <p class="text-sm text-gray-500 font-mono">{{ userInfo!.org!.domain }}</p>
            </div>
            <div class="flex items-center gap-3">
              <span v-if="userInfo!.org!.verified" class="bg-green-900/60 text-green-400 text-xs px-2 py-1 rounded">Verified</span>
              <span v-else class="bg-yellow-900/60 text-yellow-400 text-xs px-2 py-1 rounded">Unverified</span>
              <button @click="logout" class="text-xs text-gray-500 hover:text-gray-300 transition-colors cursor-pointer">Logout</button>
            </div>
          </div>
        </div>

        <!-- DIDs -->
        <div v-if="userInfo!.dids && Object.keys(userInfo!.dids).length > 0" class="bg-gray-900 rounded-lg border border-gray-800 p-6 space-y-4">
          <h3 class="text-sm font-semibold text-gray-200">Decentralized Identifiers</h3>
          <div class="space-y-2">
            <div
              v-for="(value, method) in userInfo!.dids" :key="method"
              class="flex items-center gap-3 bg-gray-950 rounded-lg px-4 py-2.5 border border-gray-800"
            >
              <span class="text-xs font-mono text-purple-400 w-16 shrink-0">{{ method }}</span>
              <span class="text-xs font-mono text-gray-400 truncate flex-1">{{ value }}</span>
              <button
                @click="copyText(value as string, method as string)"
                class="text-xs text-gray-600 hover:text-gray-300 transition-colors shrink-0 cursor-pointer"
              >
                {{ copied === method ? 'Copied' : 'Copy' }}
              </button>
            </div>
          </div>
        </div>

        <!-- API Keys -->
        <div class="bg-gray-900 rounded-lg border border-gray-800 p-6 space-y-4">
          <h3 class="text-sm font-semibold text-gray-200">API Keys</h3>

          <!-- Created key banner -->
          <div v-if="createdKey" class="bg-green-900/20 border border-green-800 rounded-lg px-4 py-3 space-y-2">
            <p class="text-xs text-green-400">New key created. Copy it now — it won't be shown again.</p>
            <div class="flex items-center gap-2">
              <code class="text-sm font-mono text-green-300 bg-gray-950 px-3 py-1 rounded flex-1">{{ createdKey }}</code>
              <button @click="copyText(createdKey, 'createdkey')" class="text-xs bg-green-800 text-green-200 px-3 py-1 rounded hover:bg-green-700 transition-colors cursor-pointer">
                {{ copied === 'createdkey' ? 'Copied' : 'Copy' }}
              </button>
            </div>
          </div>

          <!-- Key list -->
          <div class="space-y-2">
            <div
              v-for="k in orgKeys" :key="k.id"
              class="flex items-center gap-3 bg-gray-950 rounded-lg px-4 py-2.5 border border-gray-800"
            >
              <span class="text-xs font-mono text-gray-500 w-24 shrink-0">{{ k.api_key }}</span>
              <span :class="['text-xs px-2 py-0.5 rounded', k.role === 'admin' ? 'bg-purple-900/60 text-purple-400' : 'bg-blue-900/60 text-blue-400']">
                {{ k.role }}
              </span>
              <span v-if="k.email" class="text-xs text-gray-500">{{ k.email }}</span>
              <span class="flex-1" />
              <span v-if="k.revoked" class="text-xs text-red-500">revoked</span>
              <span v-else-if="apiKey.startsWith(k.api_key.replace('...', ''))" class="text-xs text-green-500">active</span>
              <button
                v-else
                @click="handleRevoke(k.id)"
                class="text-xs text-gray-600 hover:text-red-400 transition-colors cursor-pointer"
              >Revoke</button>
            </div>
            <p v-if="orgKeys.length === 0" class="text-xs text-gray-600">No API keys.</p>
          </div>

          <!-- Create key form -->
          <div class="border-t border-gray-800 pt-4 space-y-3">
            <p class="text-xs text-gray-400">Create new API key</p>
            <div class="flex gap-3">
              <input
                v-model="newKeyEmail"
                placeholder="email (optional)"
                class="bg-gray-950 border border-gray-700 rounded-lg px-3 py-2 text-sm flex-1 focus:outline-none focus:border-gray-500"
              />
              <select v-model="newKeyRole" class="bg-gray-950 border border-gray-700 rounded-lg px-3 py-2 text-sm">
                <option value="attester">attester</option>
                <option value="admin">admin</option>
              </select>
              <button @click="handleCreateKey" class="bg-gray-800 hover:bg-gray-700 text-white text-sm px-4 py-2 rounded-lg transition-colors cursor-pointer">
                Create
              </button>
            </div>
          </div>
        </div>
      </template>
    </template>

    <!-- ═══════════════════ NOT AUTHENTICATED ═══════════════════ -->
    <template v-else>

      <!-- Existing key login -->
      <div class="bg-gray-900 rounded-lg border border-gray-800 p-6 space-y-4">
        <h3 class="text-sm font-semibold text-gray-200">Already have an API key?</h3>
        <div class="flex gap-3">
          <input
            v-model="apiKey"
            placeholder="r3l_..."
            class="bg-gray-950 border border-gray-700 rounded-lg px-3 py-2 text-sm font-mono flex-1 focus:outline-none focus:border-gray-500"
          />
          <button @click="loadUser()" :disabled="loading" class="bg-gray-800 hover:bg-gray-700 text-white text-sm px-5 py-2 rounded-lg transition-colors disabled:opacity-50 cursor-pointer">
            {{ loading ? 'Loading...' : 'Login' }}
          </button>
        </div>
      </div>

      <!-- Tabbed auth methods -->
      <div v-if="!pendingVerify" class="bg-gray-900 rounded-lg border border-gray-800 overflow-hidden">
        <div class="flex border-b border-gray-800">
          <button @click="authTab = 'email'" :class="tabClass('email')">Email</button>
          <button @click="authTab = 'wallet'" :class="tabClass('wallet')">Wallet</button>
          <button @click="authTab = 'org'" :class="tabClass('org')">Organization</button>
        </div>

        <div class="p-6">

          <!-- ── Email tab ── -->
          <div v-if="authTab === 'email'" class="space-y-4">
            <p class="text-sm text-gray-400">Verify your email to sign in or create an account.</p>

            <div v-if="emailError" class="bg-red-900/30 border border-red-800 rounded-lg px-4 py-2.5 text-xs text-red-300">
              {{ emailError }}
            </div>

            <template v-if="!emailPending">
              <div>
                <label class="text-xs text-gray-500 block mb-1">Email address</label>
                <input v-model="emailAddr" placeholder="you@example.com" class="w-full bg-gray-950 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-gray-500" @keyup.enter="handleEmailStart()" />
              </div>
              <button @click="handleEmailStart" :disabled="loading || !emailAddr" class="bg-purple-700 hover:bg-purple-600 text-white text-sm px-5 py-2.5 rounded-lg transition-colors disabled:opacity-50 cursor-pointer">
                {{ loading ? 'Sending...' : 'Send Code' }}
              </button>
            </template>

            <template v-else>
              <p class="text-sm text-gray-400">
                Enter the 6-digit code sent to <strong class="text-gray-200">{{ emailAddr }}</strong>
              </p>
              <div v-if="emailDevCode" class="bg-yellow-900/20 border border-yellow-800 rounded-lg px-4 py-2 text-xs text-yellow-400">
                Dev mode — code: <strong>{{ emailDevCode }}</strong>
              </div>
              <template v-if="!emailLocked">
                <input
                  v-model="emailCode"
                  placeholder="000000"
                  maxlength="6"
                  class="w-40 bg-gray-950 border border-gray-700 rounded-lg px-3 py-2 text-lg font-mono tracking-widest text-center focus:outline-none focus:border-gray-500"
                  @keyup.enter="emailCode.length === 6 && handleEmailVerify()"
                />
                <div class="flex items-center gap-3">
                  <button @click="handleEmailVerify" :disabled="loading || emailCode.length !== 6" class="bg-purple-700 hover:bg-purple-600 text-white text-sm px-5 py-2 rounded-lg transition-colors disabled:opacity-50 cursor-pointer">
                    {{ loading ? 'Verifying...' : 'Verify Code' }}
                  </button>
                  <button @click="cancelEmail" class="text-sm text-gray-500 hover:text-gray-300 transition-colors cursor-pointer">Cancel</button>
                </div>
              </template>
              <template v-else>
                <p class="text-xs text-gray-500">Too many attempts. Please start over.</p>
                <button @click="cancelEmail" class="text-sm text-gray-500 hover:text-gray-300 transition-colors cursor-pointer">Start over</button>
              </template>
            </template>
          </div>

          <!-- ── Wallet tab ── -->
          <div v-if="authTab === 'wallet'" class="space-y-4">
            <p class="text-sm text-gray-400">Sign a challenge with your Solana wallet to sign in or create an account.</p>

            <div v-if="walletError" class="bg-red-900/30 border border-red-800 rounded-lg px-4 py-2.5 text-xs text-red-300">
              {{ walletError }}
            </div>

            <template v-if="walletMode === 'choose'">
              <div class="flex gap-3">
                <button v-if="hasPhantom" @click="walletMode = 'phantom'; connectPhantom()" class="bg-purple-700 hover:bg-purple-600 text-white text-sm px-5 py-2.5 rounded-lg transition-colors cursor-pointer">
                  Connect Phantom
                </button>
                <button @click="walletMode = 'manual'; fetchChallenge()" class="bg-gray-800 hover:bg-gray-700 text-white text-sm px-5 py-2.5 rounded-lg transition-colors cursor-pointer">
                  Enter Key Manually
                </button>
              </div>
              <p v-if="!hasPhantom" class="text-xs text-gray-600">Phantom wallet not detected. Use manual entry instead.</p>
            </template>

            <!-- Phantom flow -->
            <template v-if="walletMode === 'phantom' && walletConnected">
              <div class="space-y-3">
                <div class="bg-gray-950 rounded-lg px-4 py-2.5 border border-gray-800">
                  <span class="text-xs text-gray-500">Connected: </span>
                  <span class="text-xs font-mono text-gray-300">{{ walletPubkey }}</span>
                </div>
                <div v-if="walletChallenge" class="bg-gray-950 rounded-lg px-4 py-2.5 border border-gray-800">
                  <span class="text-xs text-gray-500">Challenge: </span>
                  <span class="text-xs font-mono text-gray-300">{{ walletChallenge }}</span>
                </div>
                <button @click="signAndVerifyPhantom" :disabled="walletLoading || !walletChallenge" class="bg-purple-700 hover:bg-purple-600 text-white text-sm px-5 py-2 rounded-lg transition-colors disabled:opacity-50 cursor-pointer">
                  {{ walletLoading ? 'Signing...' : 'Sign & Verify' }}
                </button>
              </div>
            </template>

            <!-- Manual flow -->
            <template v-if="walletMode === 'manual'">
              <div class="space-y-3">
                <div v-if="walletChallenge" class="bg-gray-950 rounded-lg px-4 py-2.5 border border-gray-800">
                  <p class="text-xs text-gray-500 mb-1">Sign this message with your wallet:</p>
                  <div class="flex items-center gap-2">
                    <code class="text-sm font-mono text-yellow-400 flex-1">{{ walletChallenge }}</code>
                    <button @click="copyText(walletChallenge, 'challenge')" class="text-xs text-gray-600 hover:text-gray-300 transition-colors shrink-0 cursor-pointer">
                      {{ copied === 'challenge' ? 'Copied' : 'Copy' }}
                    </button>
                  </div>
                </div>
                <div>
                  <label class="text-xs text-gray-500 block mb-1">Public key (base58)</label>
                  <input v-model="manualPubkey" placeholder="Base58 public key" class="w-full bg-gray-950 border border-gray-700 rounded-lg px-3 py-2 text-sm font-mono focus:outline-none focus:border-gray-500" />
                </div>
                <div>
                  <label class="text-xs text-gray-500 block mb-1">Signature (base58)</label>
                  <input v-model="manualSignature" placeholder="Base58 signature" class="w-full bg-gray-950 border border-gray-700 rounded-lg px-3 py-2 text-sm font-mono focus:outline-none focus:border-gray-500" />
                </div>
                <div class="flex items-center gap-3">
                  <button @click="handleManualWallet" :disabled="walletLoading || !manualPubkey || !manualSignature" class="bg-purple-700 hover:bg-purple-600 text-white text-sm px-5 py-2 rounded-lg transition-colors disabled:opacity-50 cursor-pointer">
                    {{ walletLoading ? 'Verifying...' : 'Verify' }}
                  </button>
                  <button @click="walletMode = 'choose'" class="text-sm text-gray-500 hover:text-gray-300 transition-colors cursor-pointer">Back</button>
                </div>
              </div>
            </template>
          </div>

          <!-- ── Organization tab ── -->
          <div v-if="authTab === 'org'" class="space-y-5">
            <p class="text-sm text-gray-400">Register your domain or log in to an existing organization.</p>

            <div class="space-y-3">
              <div>
                <label class="text-xs text-gray-500 block mb-1">Domain</label>
                <input v-model="regDomain" placeholder="example.com" class="w-full bg-gray-950 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-gray-500" />
              </div>
              <div>
                <label class="text-xs text-gray-500 block mb-1">Organization name (optional)</label>
                <input v-model="regName" placeholder="Example Inc." class="w-full bg-gray-950 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-gray-500" />
              </div>
              <div>
                <label class="text-xs text-gray-500 block mb-1">Verification method</label>
                <div class="flex gap-4">
                  <label class="flex items-center gap-2 text-sm text-gray-400 cursor-pointer">
                    <input type="radio" v-model="regMethod" value="dns" class="accent-purple-500" /> DNS TXT record
                  </label>
                  <label class="flex items-center gap-2 text-sm text-gray-400 cursor-pointer">
                    <input type="radio" v-model="regMethod" value="email" class="accent-purple-500" /> Email code
                  </label>
                </div>
              </div>
              <div v-if="regMethod === 'email'">
                <label class="text-xs text-gray-500 block mb-1">Admin email (must be @{{ regDomain || 'domain' }})</label>
                <input v-model="regEmail" :placeholder="`admin@${regDomain || 'example.com'}`" class="w-full bg-gray-950 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-gray-500" />
              </div>
            </div>

            <button @click="handleOrgRegister" :disabled="loading || !regDomain" class="bg-purple-700 hover:bg-purple-600 text-white text-sm px-5 py-2.5 rounded-lg transition-colors disabled:opacity-50 cursor-pointer">
              {{ loading ? 'Registering...' : 'Register' }}
            </button>
          </div>

        </div>
      </div>

      <!-- DNS verification pending -->
      <div v-if="pendingVerify && pendingVerify.method === 'dns'" class="bg-gray-900 rounded-lg border border-purple-800/40 p-6 space-y-4">
        <h3 class="text-sm font-semibold text-purple-400">DNS Verification</h3>
        <p class="text-sm text-gray-400">Add this TXT record to <strong class="text-gray-200">{{ pendingVerify.domain }}</strong>:</p>
        <div class="flex items-center gap-2 bg-gray-950 rounded-lg px-4 py-3 border border-gray-800">
          <code class="text-sm font-mono text-yellow-400 flex-1 break-all">{{ pendingVerify.txt_value }}</code>
          <button @click="copyText(pendingVerify.txt_value, 'dns')" class="text-xs text-gray-600 hover:text-gray-300 transition-colors shrink-0 cursor-pointer">
            {{ copied === 'dns' ? 'Copied' : 'Copy' }}
          </button>
        </div>
        <div v-if="orgVerifyError" class="bg-red-900/30 border border-red-800 rounded-lg px-4 py-2.5 text-xs text-red-300">
          {{ orgVerifyError }}
        </div>
        <p class="text-xs text-gray-500">After adding the record, click verify. DNS propagation may take a few minutes.</p>
        <div class="flex gap-3">
          <button @click="handleOrgVerifyDns" :disabled="loading" class="bg-purple-700 hover:bg-purple-600 text-white text-sm px-5 py-2 rounded-lg transition-colors disabled:opacity-50 cursor-pointer">
            {{ loading ? 'Checking...' : 'Verify DNS' }}
          </button>
          <button @click="cancelOrg" class="text-sm text-gray-500 hover:text-gray-300 transition-colors cursor-pointer">Cancel</button>
        </div>
      </div>

      <!-- Org email verification pending -->
      <div v-if="pendingVerify && pendingVerify.method === 'email'" class="bg-gray-900 rounded-lg border border-purple-800/40 p-6 space-y-4">
        <h3 class="text-sm font-semibold text-purple-400">Email Verification</h3>
        <p class="text-sm text-gray-400">
          Enter the 6-digit code sent to <strong class="text-gray-200">{{ pendingVerify.email }}</strong>
        </p>
        <div v-if="pendingVerify.dev_code" class="bg-yellow-900/20 border border-yellow-800 rounded-lg px-4 py-2 text-xs text-yellow-400">
          Dev mode — code: <strong>{{ pendingVerify.dev_code }}</strong>
        </div>
        <div v-if="orgVerifyError" class="bg-red-900/30 border border-red-800 rounded-lg px-4 py-2.5 text-xs text-red-300">
          {{ orgVerifyError }}
        </div>
        <div v-if="orgResendSuccess" class="bg-green-900/20 border border-green-800 rounded-lg px-4 py-2.5 text-xs text-green-400">
          New code sent. Check your inbox.
        </div>
        <template v-if="!orgLocked">
          <input
            v-model="orgEmailCode"
            placeholder="000000"
            maxlength="6"
            class="w-40 bg-gray-950 border border-gray-700 rounded-lg px-3 py-2 text-lg font-mono tracking-widest text-center focus:outline-none focus:border-gray-500"
            @keyup.enter="orgEmailCode.length === 6 && handleOrgVerifyEmail()"
          />
          <div class="flex items-center gap-3">
            <button @click="handleOrgVerifyEmail" :disabled="loading || orgEmailCode.length !== 6" class="bg-purple-700 hover:bg-purple-600 text-white text-sm px-5 py-2 rounded-lg transition-colors disabled:opacity-50 cursor-pointer">
              {{ loading ? 'Verifying...' : 'Verify Code' }}
            </button>
            <button @click="handleOrgResend" :disabled="orgResending" class="text-sm text-gray-500 hover:text-gray-300 transition-colors cursor-pointer disabled:opacity-50">
              {{ orgResending ? 'Sending...' : 'Resend code' }}
            </button>
            <button @click="cancelOrg" class="text-sm text-gray-500 hover:text-gray-300 transition-colors cursor-pointer">Cancel</button>
          </div>
        </template>
        <template v-else>
          <p class="text-xs text-gray-500">Request a new code to try again.</p>
          <div class="flex items-center gap-3">
            <button @click="handleOrgResend" :disabled="orgResending" class="bg-purple-700 hover:bg-purple-600 text-white text-sm px-5 py-2 rounded-lg transition-colors disabled:opacity-50 cursor-pointer">
              {{ orgResending ? 'Sending...' : 'Send new code' }}
            </button>
            <button @click="cancelOrg" class="text-sm text-gray-500 hover:text-gray-300 transition-colors cursor-pointer">Cancel</button>
          </div>
        </template>
      </div>

    </template>
  </div>
</template>
