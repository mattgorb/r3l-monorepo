#!/bin/bash
set -e

BASE_URL="https://d2ok2lga3g52rl.cloudfront.net"
API_KEY="${API_KEY:-r3l_REPLACE_ME}"
DIR="$(cd "$(dirname "$0")/../../../data/test_files" && pwd)"

echo "=== Batch 1/2: 8 files ==="
curl -s -X POST "$BASE_URL/api/v1/attest-content/batch" \
  -H "X-API-Key: $API_KEY" \
  -F "file=@$DIR/a_test.png" \
  -F "file=@$DIR/a2.png" \
  -F "file=@$DIR/Firefly_Gemini.png" \
  -F "file=@$DIR/1507713048_google_pixel.jpg" \
  -F "file=@$DIR/google_pixelgoogle.jpg" \
  -F "file=@$DIR/adobe-20220124-CACA.jpg" \
  -F "file=@$DIR/adobe-20220124-CAIAIIICAICIICAIICICA.jpg" \
  -F "file=@$DIR/adobe-20240110-single_manifest_store.pdf" | python3 -m json.tool

echo ""
echo "=== Batch 2/2: 7 files ==="
curl -s -X POST "$BASE_URL/api/v1/attest-content/batch" \
  -H "X-API-Key: $API_KEY" \
  -F "file=@$DIR/leicaPverify_tool_01_l1002253.jpg" \
  -F "file=@$DIR/nikon-20221019-building.jpeg" \
  -F "file=@$DIR/truepic-20230212-zoetrope.mp4" \
  -F "file=@$DIR/park-bench-sunset-c2pa-max-vmh.mp4" \
  -F "file=@$DIR/b.pdf" \
  -F "file=@$DIR/lcd_x_hcpc_code.csv" \
  -F "file=@$DIR/state_x_region.csv" | python3 -m json.tool

echo ""
echo "Done — 18 files attested across 5 requests."
