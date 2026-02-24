#!/bin/bash
set -e

PROGRAM_SO="/deploy/provenance_attestation.so"
PROGRAM_ID="${PROGRAM_ID:-kK63ncGUJXCWjUqSYzcN92tW531rt4UtacJBmHWXJ16}"

if [ ! -f "$PROGRAM_SO" ]; then
  echo ""
  echo "ERROR: $PROGRAM_SO not found."
  echo ""
  echo "Build the Solana program first:"
  echo "  cd services/provenance_attestation"
  echo "  anchor build --no-idl -- --features skip-verification,skip-authority-check"
  echo ""
  exit 1
fi

# Generate a keypair if none was mounted from the host.
# This keypair gets funded at genesis by the test validator.
if [ ! -f /root/.config/solana/id.json ]; then
  mkdir -p /root/.config/solana
  solana-keygen new --no-passphrase -o /root/.config/solana/id.json --silent
  echo "Generated ephemeral keypair: $(solana-keygen pubkey /root/.config/solana/id.json)"
fi

echo "Starting validator with program $PROGRAM_ID"
exec solana-test-validator \
  --reset \
  --bind-address 0.0.0.0 \
  --bpf-program "$PROGRAM_ID" "$PROGRAM_SO" \
  "$@"
