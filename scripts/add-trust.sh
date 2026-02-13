#!/usr/bin/env bash
#
# Add a trust record to the curated or official trust directory.
#
# Usage:
#   ./scripts/add-trust.sh <list> <name> <pem-file-or-image>
#
# Arguments:
#   list    "official" or "curated"
#   name    short slug for the file, e.g. "openai-truepic" → openai-truepic.pem
#   source  path to a .pem file OR a C2PA-signed image (requires c2patool)
#
# Examples:
#   # Add a PEM file directly:
#   ./scripts/add-trust.sh curated my-org /path/to/certs.pem
#
#   # Extract certs from a signed image (requires c2patool):
#   ./scripts/add-trust.sh curated adobe-firefly /path/to/signed-image.png
#
set -euo pipefail

TRUST_DIR="$(cd "$(dirname "$0")/.." && pwd)/data/trust"

usage() {
    echo "Usage: $0 <official|curated> <name> <pem-file-or-image>"
    echo ""
    echo "  list    'official' or 'curated'"
    echo "  name    short slug for the output file (e.g. 'openai-truepic')"
    echo "  source  path to a .pem file, or a C2PA-signed image (needs c2patool)"
    exit 1
}

if [ $# -ne 3 ]; then
    usage
fi

LIST="$1"
NAME="$2"
SOURCE="$3"

if [ "$LIST" != "official" ] && [ "$LIST" != "curated" ]; then
    echo "Error: list must be 'official' or 'curated', got '$LIST'"
    exit 1
fi

if [ ! -f "$SOURCE" ]; then
    echo "Error: source file not found: $SOURCE"
    exit 1
fi

DEST_DIR="$TRUST_DIR/$LIST"
DEST="$DEST_DIR/$NAME.pem"

if [ -f "$DEST" ]; then
    echo "Error: $DEST already exists. Remove it first or choose a different name."
    exit 1
fi

mkdir -p "$DEST_DIR"

# Detect if source is a PEM or an image
if head -1 "$SOURCE" | grep -q "BEGIN CERTIFICATE\|^#"; then
    # It's already a PEM file — copy it
    cp "$SOURCE" "$DEST"
    echo "Copied PEM to $DEST"
else
    # Treat as an image — extract certs with c2patool
    if ! command -v c2patool &>/dev/null; then
        echo "Error: c2patool not found. Install it or provide a .pem file instead."
        echo "  cargo install c2patool"
        exit 1
    fi

    BASENAME="$(basename "$SOURCE")"

    echo "# Source: $BASENAME" > "$DEST"
    echo "# Extracted by add-trust.sh on $(date -u +%Y-%m-%d)" >> "$DEST"

    c2patool "$SOURCE" --certs >> "$DEST" 2>/dev/null

    if ! grep -q "BEGIN CERTIFICATE" "$DEST"; then
        rm -f "$DEST"
        echo "Error: no certificates found in $SOURCE"
        exit 1
    fi

    echo "Extracted certs from $BASENAME to $DEST"
fi

# Show summary
CERT_COUNT=$(grep -c "BEGIN CERTIFICATE" "$DEST")
echo "  $CERT_COUNT certificate(s) in $DEST"
