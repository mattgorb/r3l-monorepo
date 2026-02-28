#!/bin/bash
set -e

BASE_URL="https://d2ok2lga3g52rl.cloudfront.net"
API_KEY="r3l_6370ac495d6f7aea389a3a0b4f2c39e420036c1b266e9879"

# Verify all identities at once
echo "=== Sending verification (email + wallet + org) ==="
curl -s -X POST "$BASE_URL/api/v1/verify-identity" \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $API_KEY" \
  -d '{
    "email": "matthewgorbett@gmail.com",
    "wallet_pubkey": "6djoBBfibFKV4LsepDN3u8D8E159wP1NzWnKvwbHn2Cn",
    "wallet_message": "R3L-verify:my-app",
    "wallet_signature": "TiadsSoHLiLRHvbt39fYej7bK6dnpfa9Qv2gACVU1T2c4T6z19hgas36zbwsWfrRcMQMxtQ4hpLczeQFn4SZdwH",
    "org_domain": "gmail.com"
  }' | python3 -m json.tool

echo ""
echo "=== Confirm email code ==="
echo "Replace CODE with the code from your email inbox:"
echo "  API_KEY=$API_KEY ./2_verify_identity_confirm.sh CODE"
