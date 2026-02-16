# Prove Job Polling — Implementation Plan

## Overview

Replace the synchronous POST `/api/prove` (which blocks for minutes) with an async job model:
- **POST /api/prove** → starts job, returns `{ job_id }` immediately
- **GET /api/prove/:id** → returns `{ status, logs[], result? }`
- Frontend polls every 2s, displays log lines in real-time

## Architecture

```
Browser                    API                         Prover Binary
  │                         │                              │
  ├─ POST /prove (file) ──▶ │                              │
  │                         ├─ save file, spawn process ──▶│
  │◀── { job_id } ─────────┤                              │
  │                         │◀── stdout/stderr lines ──────┤
  ├─ GET /prove/:id ──────▶ │                              │
  │◀── { status, logs } ───┤                              │
  │         ...             │         ...                  │
  ├─ GET /prove/:id ──────▶ │                              │
  │◀── { status: "done",   │◀── exit 0 ──────────────────┤
  │      logs, result }     │                              │
```

## Job Store (in-memory HashMap)

```rust
struct ProveJob {
    status: JobStatus,          // Running | Done | Failed
    logs: Vec<String>,          // stdout/stderr lines captured so far
    result: Option<ProveResponse>,
    error: Option<String>,
    created_at: Instant,
}

// Shared state added to AppState:
jobs: Arc<RwLock<HashMap<String, ProveJob>>>
```

**Why HashMap, not Redis:** Single server, single process. Swapping to Redis later only requires changing the store implementation — the API contract stays identical.

**Cleanup:** Prune completed jobs older than 30 minutes on each new POST to prevent unbounded growth.

## Files Changed

### 1. `services/api/src/main.rs`
- Add `jobs: Arc<RwLock<HashMap<String, ProveJob>>>` to `AppState`
- Add route: `GET /api/prove/:id` → `routes::prove::status`

### 2. `services/api/src/routes/prove.rs`
- Add `ProveJob`, `JobStatus` types
- **POST handler (`prove`)**: save file to temp, insert job as `Running`, spawn a `tokio::spawn` task that:
  - Runs verify (blocking)
  - Spawns prover with `.spawn()` (not `.output()`)
  - Reads stdout/stderr via `BufReader` + `lines()`, appends each to `job.logs`
  - On completion: reads JSON sidecar, sets `status = Done` + `result`
  - On failure: sets `status = Failed` + `error`
  - Returns `{ job_id }` immediately
- **GET handler (`status`)**: read job from store, return current state
  - Accept `?after=N` query param — only return logs after index N (so frontend doesn't re-fetch old lines)

### 3. `services/api/Cargo.toml`
- Add `uuid = { version = "1", features = ["v4"] }` for job IDs

### 4. `services/web/src/api.ts`
- `startProve(file: File) → Promise<{ job_id: string }>` — POST, returns immediately
- `pollProve(jobId: string, after?: number) → Promise<ProveStatus>` — GET with ?after=N
- `proveFile(file, onLog)` — convenience wrapper that starts + polls in a loop, calls `onLog(line)` for each new line

### 5. `services/web/src/types.ts`
- Add `ProveStatus` type:
  ```ts
  interface ProveStatus {
    status: 'running' | 'done' | 'failed'
    logs: string[]
    result?: ProveResponse
    error?: string
  }
  ```

### 6. `services/web/src/components/ProofStatus.vue`
- Add `logs: ref<string[]>([])` — displayed in a scrollable log box
- On "Generate Proof" click: call `startProve`, then `setInterval` polling every 2s
- Each poll: append new log lines, auto-scroll to bottom
- When `status === 'done'`: clear interval, emit result
- When `status === 'failed'`: clear interval, show error

## API Contract

### POST /api/prove
**Request:** multipart/form-data with `file` field
**Response:** `{ "job_id": "abc-123" }`

### GET /api/prove/:id?after=0
**Response (running):**
```json
{
  "status": "running",
  "logs": ["[verify] has_c2pa: true", "[sp1] generating proof..."]
}
```

**Response (done):**
```json
{
  "status": "done",
  "logs": ["[sp1] proof complete"],
  "result": { "proof": "0x...", "public_outputs": "0x...", "verify_output": {...} }
}
```

**Response (failed):**
```json
{
  "status": "failed",
  "logs": ["[sp1] error: ..."],
  "error": "prover exited with code 1"
}
```

## Future: Redis Upgrade Path

When needed, replace `Arc<RwLock<HashMap>>` with a Redis client:
- `HSET prove:job:{id} status running`
- `RPUSH prove:job:{id}:logs "line"`
- `HSET prove:job:{id} result "{json}"`
- Poll via `LRANGE prove:job:{id}:logs after -1`

No API or frontend changes required.
