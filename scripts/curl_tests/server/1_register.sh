#!/bin/bash
set -e

BASE_URL="https://d2ok2lga3g52rl.cloudfront.net"

curl -s -X POST "$BASE_URL/api/v1/register" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "matthewgorbett@gmail.com",
    "name": "My App"
  }' | python3 -m json.tool
