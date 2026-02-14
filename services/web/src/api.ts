import axios from 'axios'
import type { VerifyOutput, ProveResponse, SubmitResponse, AttestationResponse } from './types'

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

export async function lookupAttestation(hash: string): Promise<AttestationResponse> {
  const { data } = await client.get<AttestationResponse>(`/attestation/${hash}`)
  return data
}
