# Deploying R3L to AWS

App Runner + RDS Postgres + CloudFront. Estimated cost: **~$25/mo** ($13 RDS + $7 App Runner + CloudFront free tier).

## Prerequisites

- AWS CLI configured (`aws configure`)
- Terraform >= 1.6 (`brew install terraform`)
- Docker running
- Solana keypair at `~/.config/solana/id.json` (the payer account for on-chain transactions)

If you don't have a Solana keypair:
```bash
solana-keygen new
```

## 1. Configure

```bash
cd infra/r3l-app

# Copy the example and fill in your values
cp terraform.tfvars.example terraform.tfvars
```

Edit `terraform.tfvars`:
```hcl
aws_region     = "us-east-1"
solana_rpc_url = "https://api.devnet.solana.com"  # or mainnet-beta
db_password    = "your_strong_password_here"       # pick something random
program_id     = "63jq6M3t5NafYWcADqLDCLnhd5qPfEmCUcaA9iWh5YWz"
```

## 2. Create Infrastructure

```bash
terraform init
terraform plan      # review what will be created
terraform apply     # type "yes" to confirm
```

This creates:
- VPC with private subnets
- RDS Postgres (db.t4g.micro, 20GB)
- ECR repository
- App Runner service (1 vCPU, 2GB RAM)
- VPC connector (App Runner → RDS)
- CloudFront distribution

Takes ~10 minutes (RDS is the slow part).

## 3. Deploy the App

```bash
./deploy.sh
```

This builds the Docker image (verifier + frontend + Python API), pushes to ECR, and App Runner auto-deploys.

First build takes ~5 minutes (Rust compilation). Subsequent builds use Docker cache.

## 4. Verify

```bash
# Get your URLs
terraform output cloudfront_url
terraform output apprunner_url

# Test health
curl https://YOUR_CLOUDFRONT_URL/api/health

# Test query API
curl https://YOUR_CLOUDFRONT_URL/api/v1/query/abc123
```

## 5. Fund the Solana Keypair

The keypair baked into the Docker image needs SOL to pay for transactions.

**Devnet:**
```bash
solana airdrop 2 --url https://api.devnet.solana.com
```

**Mainnet:**
Send SOL to the pubkey shown by:
```bash
solana-keygen pubkey ~/.config/solana/id.json
```

## Cost Breakdown

| Resource | Monthly Cost |
|----------|-------------|
| RDS db.t4g.micro | ~$13 (free tier eligible for 12 months) |
| App Runner (1 vCPU, 2GB, always-on) | ~$7 |
| CloudFront (<1TB) | Free |
| ECR (<500MB) | Free |
| VPC / networking | Free |
| **Total** | **~$20-25/mo** |

## Managing the Deployment

### Pause (stop billing for App Runner)
```bash
terraform output -raw pause_command | bash
```

### Resume
```bash
terraform output -raw resume_command | bash
```

### Redeploy after code changes
```bash
./deploy.sh
```

### View logs
```bash
aws apprunner list-services --region us-east-1
# Get service ARN, then:
aws apprunner describe-service --service-arn <ARN>
```

### Tear down everything
```bash
terraform destroy
```

## Architecture

```
User → CloudFront (HTTPS + caching)
         ↓
       App Runner (Python API + verifier binary + Vue frontend)
         ↓                    ↓
       RDS Postgres         Solana RPC (devnet/mainnet)
       (attestation DB)     (on-chain PDAs)
```

CloudFront caches static assets (Vue SPA) at the edge. API routes (`/api/*`) pass through with no caching.

## Troubleshooting

**App Runner won't start:** Check the health check is returning 200 at `/api/health`. The DB connection might be failing — verify the VPC connector is attached and security groups allow port 5432.

**"Connection refused" to RDS:** App Runner needs the VPC connector to reach RDS in the private subnet. Verify `terraform output` shows the VPC connector ARN.

**Solana transactions fail:** Make sure the keypair has SOL for transaction fees. On devnet, use `solana airdrop`. Check `SOLANA_RPC_URL` and `PROGRAM_ID` are correct.

**CloudFront returns old content:** Invalidate the cache:
```bash
aws cloudfront create-invalidation \
  --distribution-id $(terraform output -raw cloudfront_distribution_id) \
  --paths "/*"
```
