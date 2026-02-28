#!/bin/bash
set -e

BASE_URL="https://d2ok2lga3g52rl.cloudfront.net"
API_KEY="r3l_56fdd66aedb3bd132ee38d194b2c1a87643a208e1851dc04"
CODE="${1:?Usage: ./2_verify_identity_confirm.sh CODE}"

curl -s -X POST "$BASE_URL/api/v1/verify-identity" \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $API_KEY" \
  -d "{
    \"email\": \"matthewgorbett@gmail.com\",
    \"code\": \"$CODE\"
  }" | python3 -m json.tool
