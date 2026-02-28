#!/bin/bash
set -e

BASE_URL="http://localhost:3001"
API_KEY="${API_KEY:-r3l_REPLACE_ME}"
DIR="$(cd "$(dirname "$0")/../../../data/test_files" && pwd)"

echo "=== Single file 1/3: chatgpt.png ==="
curl -s -X POST "$BASE_URL/api/v1/attest-content" \
  -H "X-API-Key: $API_KEY" \
  -F "file=@$DIR/chatgpt.png" | python3 -m json.tool

echo ""
echo "=== Single file 2/3: adobe-20220124-CICA.jpg ==="
curl -s -X POST "$BASE_URL/api/v1/attest-content" \
  -H "X-API-Key: $API_KEY" \
  -F "file=@$DIR/adobe-20220124-CICA.jpg" | python3 -m json.tool

echo ""
echo "=== Single file 3/3: truepic-20230212-camera.jpg ==="
curl -s -X POST "$BASE_URL/api/v1/attest-content" \
  -H "X-API-Key: $API_KEY" \
  -F "file=@$DIR/truepic-20230212-camera.jpg" | python3 -m json.tool
