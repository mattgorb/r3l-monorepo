# R3L App — App Runner + CloudFront

Full app (API + frontend) on AWS App Runner with CloudFront for HTTPS and edge caching.


## First-time Setup

```bash
cd infra/r3l-app
terraform init
terraform apply
./deploy.sh

# First time only — recreate App Runner now that ECR has an image
terraform apply -replace=aws_apprunner_service.r3l_app

```

This creates: ECR repo, App Runner service, CloudFront distribution.

## Deploy

```bash
# Build and push Docker image to ECR
d

# App Runner auto-deploys on image push
```

## Start / Stop

```bash
# Pause (stop paying for compute)
terraform output -raw pause_command | bash

# Resume
terraform output -raw resume_command | bash
```

## URLs

```bash
# App Runner direct URL (HTTPS)
terraform output apprunner_url

# CloudFront URL (HTTPS + edge caching)
terraform output cloudfront_url
```

## Custom Domain (later)

1. Register domain
2. Add `aliases` and ACM cert to CloudFront in `main.tf`
3. Point DNS CNAME to CloudFront distribution
