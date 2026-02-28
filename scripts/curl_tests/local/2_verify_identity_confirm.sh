#!/bin/bash
set -e

BASE_URL="http://localhost:3001"
API_KEY="${API_KEY:-r3l_REPLACE_ME}"
CODE="${1:?Usage: ./2_verify_identity_confirm.sh CODE}"

curl -s -X POST "$BASE_URL/api/v1/verify-identity" \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $API_KEY" \
  -d "{
    \"email\": \"matthewgorbett@gmail.com\",
    \"code\": \"$CODE\"
  }" | python3 -m json.tool
