#!/usr/bin/env bash
set -euo pipefail

# Deploy R3L app to App Runner via ECR
# Usage: ./deploy.sh [REGION]

REGION="${1:-us-east-1}"
REPO_NAME="r3l-app"
PROJECT_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
KEYPAIR_PATH="${SOLANA_KEYPAIR_PATH:-$HOME/.config/solana/id.json}"

# Validate keypair exists
if [ ! -f "$KEYPAIR_PATH" ]; then
  echo "ERROR: Solana keypair not found at $KEYPAIR_PATH"
  echo "       Generate one with: solana-keygen new"
  echo "       Or set SOLANA_KEYPAIR_PATH to your keypair location"
  exit 1
fi

echo "==> Getting ECR repository URL..."
ACCOUNT_ID=$(aws sts get-caller-identity --query Account --output text)
ECR_URL="${ACCOUNT_ID}.dkr.ecr.${REGION}.amazonaws.com/${REPO_NAME}"

echo "==> Logging into ECR..."
aws ecr get-login-password --region "$REGION" | \
  docker login --username AWS --password-stdin "${ACCOUNT_ID}.dkr.ecr.${REGION}.amazonaws.com"

# Copy Solana keypair into build context temporarily
cp "$KEYPAIR_PATH" "$PROJECT_ROOT/docker/.solana-keypair.json"
trap 'rm -f "$PROJECT_ROOT/docker/.solana-keypair.json"' EXIT

echo "==> Building Docker image (linux/amd64 for App Runner)..."
docker build \
  --platform linux/amd64 \
  -f "$PROJECT_ROOT/docker/api-py.Dockerfile" \
  -t "$REPO_NAME:latest" \
  "$PROJECT_ROOT"

echo "==> Tagging and pushing to ECR..."
docker tag "$REPO_NAME:latest" "$ECR_URL:latest"
docker push "$ECR_URL:latest"

echo ""
echo "==> Deployed! App Runner will auto-deploy the new image."
echo "    ECR: $ECR_URL:latest"
echo ""
echo "    Useful commands:"
echo "    - View status:  aws apprunner list-services --region $REGION"
echo "    - Pause (save): terraform output -raw pause_command | bash"
echo "    - Resume:       terraform output -raw resume_command | bash"
echo "    - CloudFront:   terraform output cloudfront_url"
