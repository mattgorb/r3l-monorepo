#!/bin/bash
set -e

lsof -ti :8899 | xargs kill -9 2>/dev/null || true
lsof -ti :3001 | xargs kill -9 2>/dev/null || true

# --reset-db flag: drop and recreate the database
if [[ "$1" == "--reset-db" || "$1" == "-r" ]]; then
  echo "Resetting database..."
  docker exec r3l-postgres psql -U postgres -c "DROP DATABASE IF EXISTS r3l;" 2>/dev/null || true
  docker exec r3l-postgres psql -U postgres -c "CREATE DATABASE r3l;" 2>/dev/null || true
  echo "Database reset."
fi

# Build frontend
echo "Building frontend..."
(cd services/web && npm run build)

# Start Postgres (reuse existing container if present)
if ! docker ps --format '{{.Names}}' | grep -q '^r3l-postgres$'; then
  echo "Starting Postgres..."
  docker start r3l-postgres 2>/dev/null || \
    docker run -d --name r3l-postgres -p 5432:5432 \
      -e POSTGRES_PASSWORD=postgres -e POSTGRES_DB=r3l \
      pgvector/pgvector:pg16
  for i in $(seq 1 15); do
    if docker exec r3l-postgres pg_isready -q 2>/dev/null; then
      echo "Postgres ready."
      break
    fi
    sleep 1
  done
fi

PROGRAM_SO="services/provenance_attestation/target/deploy/provenance_attestation.so"
PROGRAM_KEYPAIR="services/provenance_attestation/target/deploy/provenance_attestation-keypair.json"

if [ ! -f "$PROGRAM_SO" ]; then
  echo "Program .so not found. Building..."
  (cd services/provenance_attestation && anchor build --no-idl -- --features skip-verification,skip-authority-check)
fi

# Generate keypair if it doesn't exist
if [ ! -f ~/.config/solana/id.json ]; then
  echo "No Solana keypair found. Generating..."
  solana-keygen new --no-passphrase
fi

cleanup() {
  echo ""
  echo "Shutting down..."
  kill $API_PID 2>/dev/null
  kill $VALIDATOR_PID 2>/dev/null
  wait $API_PID 2>/dev/null
  wait $VALIDATOR_PID 2>/dev/null
  exit 0
}
trap cleanup INT TERM

# Get program ID from the build keypair (must match declare_id! in the compiled .so)
PROGRAM_ID=$(solana address -k "$PROGRAM_KEYPAIR")
echo "Program ID: $PROGRAM_ID"

# Start validator in background with program pre-loaded
echo "Starting Solana test validator..."
solana-test-validator --reset \
  --bpf-program "$PROGRAM_ID" "$PROGRAM_SO" \
  --quiet &
VALIDATOR_PID=$!

# Wait for validator to be ready
echo "Waiting for validator..."
for i in $(seq 1 30); do
  if solana cluster-version -u http://127.0.0.1:8899 &>/dev/null; then
    echo "Validator ready."
    break
  fi
  if ! kill -0 $VALIDATOR_PID 2>/dev/null; then
    echo "Validator failed to start."
    exit 1
  fi
  sleep 1
done

# Start Python API server
echo "Starting API server (Python)..."
cd services/api-py
PROGRAM_ID="$PROGRAM_ID" \
  TRUST_DIR="../../data/trust" \
  VERIFIER_BIN="../verifier/target/release/verifier" \
  PROVER_DIR="../prover" \
  SOLANA_RPC_URL="http://127.0.0.1:8899" \
  STATIC_DIR="../web/dist" \
  DATABASE_URL="postgresql://postgres:postgres@localhost:5432/r3l" \
  python3 -m uvicorn main:app --host 0.0.0.0 --port 3001 --reload &
API_PID=$!
cd ../..

echo ""
echo "==========================================="
echo "  R3L running at http://localhost:3001"
echo "  Solana RPC at  http://localhost:8899"
echo "  Ctrl+C to stop everything"
echo "==========================================="
echo ""

wait $API_PID
