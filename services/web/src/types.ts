export interface VerifyOutput {
  path: string
  content_hash: string | null
  has_c2pa: boolean
  trust_list_match: string | null
  validation_state: string | null
  validation_error_count: number | null
  validation_codes: string[] | null
  title: string | null
  format: string | null
  digital_source_type: string | null
  claim_generator: string | null
  software_agent: string | null
  issuer: string | null
  common_name: string | null
  signing_time: string | null
  sig_algorithm: string | null
  actions: any | null
  ingredients: any | null
  manifest_store: any | null
  error: string | null
}

export interface ProveResponse {
  proof: string
  public_outputs: string
  verify_output: VerifyOutput
}

export interface SubmitResponse {
  signature: string
  attestation_pda: string
}

export interface AttestResponse {
  signature: string | null
  attestation_pda: string
  content_hash: string
  verify_output: VerifyOutput
  existing?: boolean
  email_domain?: string
  wallet_pubkey?: string
}

export interface AttestationResponse {
  content_hash: string
  has_c2pa: boolean
  trust_list_match: string
  validation_state: string
  digital_source_type: string
  issuer: string
  common_name: string
  software_agent: string
  signing_time: string
  submitted_by: string
  timestamp: number
  proof_type: string
  email_domain?: string
  wallet_pubkey?: string
  wallet_sig?: string
  verifier_version?: string
  trust_bundle_hash?: string
}

export interface AttestationListItem {
  content_hash: string
  proof_type: string
  timestamp: number
  issuer?: string
  trust_list_match?: string
  email_domain?: string
  wallet_pubkey?: string
}

export interface SimilarMatch {
  content_hash: string
  match_type: 'exact' | 'near_duplicate' | 'visual_match' | 'unrelated'
  tlsh_hash?: string | null
  tlsh_distance?: number | null
  clip_similarity?: number | null
  issuer?: string | null
  trust_list_match?: string | null
  has_c2pa?: boolean | null
  timestamp: number
}

export interface SimilarResponse {
  query_hash: string
  query_tlsh?: string | null
  matches: SimilarMatch[]
}

export interface MeResponse {
  type: 'individual' | 'org'
  // Individual fields
  name?: string
  email?: string | null
  wallet_pubkey?: string | null
  auth_method?: 'email' | 'wallet' | null
  // Org fields
  org?: OrgInfo['org']
  dids?: Record<string, string>
  role?: string
}

export interface OrgInfo {
  org: {
    id: number
    domain: string
    name: string | null
    verified: boolean
    verification_method: string | null
    admin_email: string | null
    did_web: string | null
    created_at: number
  }
  dids: Record<string, string>
}

export interface OrgKeyItem {
  id: number
  org_id: number
  email: string | null
  role: string
  api_key: string
  created_at: number
  revoked: boolean
}

