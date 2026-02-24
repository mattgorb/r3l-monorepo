import axios from 'axios'
import type {
  VerifyOutput, ProveResponse, SubmitResponse, AttestResponse, AttestationResponse,
  AttestationListItem, SimilarResponse, MeResponse, OrgInfo, OrgKeyItem,
} from './types'

const client = axios.create({ baseURL: '/api' })

export async function verifyFile(file: File): Promise<VerifyOutput> {
  const form = new FormData()
  form.append('file', file)
  const { data } = await client.post<VerifyOutput>('/verify', form)
  return data
}

export async function proveFile(file: File): Promise<ProveResponse> {
  const form = new FormData()
  form.append('file', file)
  const { data } = await client.post<ProveResponse>('/prove', form)
  return data
}

export async function submitAttestation(params: {
  content_hash: string
  proof: string
  public_inputs: string
}): Promise<SubmitResponse> {
  const { data } = await client.post<SubmitResponse>('/submit', params)
  return data
}

export async function attestFile(
  file: File,
  opts?: {
    walletPubkey?: string
    walletMessage?: string
    walletSignature?: string
  },
): Promise<AttestResponse> {
  const form = new FormData()
  form.append('file', file)
  if (opts?.walletPubkey) form.append('wallet_pubkey', opts.walletPubkey)
  if (opts?.walletMessage) form.append('wallet_message', opts.walletMessage)
  if (opts?.walletSignature) form.append('wallet_signature', opts.walletSignature)
  const { data } = await client.post<AttestResponse>('/attest', form)
  return data
}

export async function lookupAttestation(hash: string): Promise<AttestationResponse> {
  const { data } = await client.get<AttestationResponse>(`/attestation/${hash}`)
  return data
}

export async function listAttestations(): Promise<AttestationListItem[]> {
  const { data } = await client.get<AttestationListItem[]>('/attestations')
  return data
}

export async function queryVerdict(hash: string): Promise<any> {
  const { data } = await client.get(`/v1/query/${hash}`)
  return data
}

export async function searchSimilarByFile(file: File): Promise<SimilarResponse> {
  const form = new FormData()
  form.append('file', file)
  const { data } = await client.post<SimilarResponse>('/v1/similar', form)
  return data
}

export async function searchSimilarByHash(hash: string): Promise<SimilarResponse> {
  const { data } = await client.get<SimilarResponse>(`/v1/similar/${hash}`)
  return data
}

// ── Auth functions ───────────────────────────────────────────────

export async function authEmailStart(email: string): Promise<any> {
  const { data } = await client.post('/auth/email/start', { email })
  return data
}

export async function authEmailVerify(email: string, code: string): Promise<any> {
  const { data } = await client.post('/auth/email/verify', { email, code })
  return data
}

export async function authWalletChallenge(): Promise<{ challenge: string; expires_in: number }> {
  const { data } = await client.get('/auth/wallet/challenge')
  return data
}

export async function authWalletVerify(params: {
  pubkey: string; message: string; signature: string; name?: string
}): Promise<any> {
  const { data } = await client.post('/auth/wallet/verify', params)
  return data
}

export async function getMe(apiKey: string): Promise<MeResponse> {
  const { data } = await client.get<MeResponse>('/auth/me', {
    headers: { 'X-API-Key': apiKey },
  })
  return data
}

// ── Identity linking functions ───────────────────────────────────

export async function linkEmailStart(apiKey: string, email: string): Promise<any> {
  const { data } = await client.post('/auth/link/email/start', { email }, {
    headers: { 'X-API-Key': apiKey },
  })
  return data
}

export async function linkEmailVerify(apiKey: string, email: string, code: string): Promise<any> {
  const { data } = await client.post('/auth/link/email/verify', { email, code }, {
    headers: { 'X-API-Key': apiKey },
  })
  return data
}

export async function linkWallet(apiKey: string, params: {
  pubkey: string; message: string; signature: string
}): Promise<any> {
  const { data } = await client.post('/auth/link/wallet', params, {
    headers: { 'X-API-Key': apiKey },
  })
  return data
}

// ── Org functions ────────────────────────────────────────────────

export async function registerOrg(params: {
  domain: string; method: string; admin_email?: string; name?: string
}): Promise<any> {
  const { data } = await client.post('/org/register', params)
  return data
}

export async function verifyOrgDns(domain: string): Promise<any> {
  const { data } = await client.post('/org/verify/dns', { domain })
  return data
}

export async function verifyOrgEmail(email: string, code: string): Promise<any> {
  const { data } = await client.post('/org/verify/email', { email, code })
  return data
}

export async function resendOrgCode(domain: string, admin_email: string): Promise<any> {
  const { data } = await client.post('/org/resend', { domain, admin_email })
  return data
}

export async function getOrgInfo(apiKey: string): Promise<OrgInfo> {
  const { data } = await client.get<OrgInfo>('/org/info', {
    headers: { 'X-API-Key': apiKey },
  })
  return data
}

export async function createOrgKey(
  apiKey: string, params: { email?: string; role?: string },
): Promise<any> {
  const { data } = await client.post('/org/keys', params, {
    headers: { 'X-API-Key': apiKey },
  })
  return data
}

export async function listOrgKeys(apiKey: string): Promise<{ keys: OrgKeyItem[] }> {
  const { data } = await client.get<{ keys: OrgKeyItem[] }>('/org/keys', {
    headers: { 'X-API-Key': apiKey },
  })
  return data
}

export async function revokeOrgKey(apiKey: string, keyId: number): Promise<any> {
  const { data } = await client.delete(`/org/keys/${keyId}`, {
    headers: { 'X-API-Key': apiKey },
  })
  return data
}

export async function resolveDid(did: string): Promise<any> {
  const { data } = await client.get(`/did/${encodeURIComponent(did)}`)
  return data
}

