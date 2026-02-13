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
}
