terraform {
  required_version = ">= 1.6.0"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = ">= 5.0"
    }
  }
}

############################
# Variables
############################
variable "aws_region"     { default = "us-east-1" }
variable "solana_rpc_url" { default = "https://api.devnet.solana.com" }
variable "db_password"    { sensitive = true }
variable "program_id"     { default = "63jq6M3t5NafYWcADqLDCLnhd5qPfEmCUcaA9iWh5YWz" }
variable "smtp_host"      { default = "" }
variable "smtp_user"      { default = "" }
variable "smtp_pass" {
  default   = ""
  sensitive = true
}
variable "smtp_from"      { default = "" }

provider "aws" {
  region = var.aws_region
}

############################
# VPC
############################
resource "aws_vpc" "main" {
  cidr_block           = "10.0.0.0/16"
  enable_dns_support   = true
  enable_dns_hostnames = true
  tags = { Name = "r3l-vpc", Project = "r3l" }
}

resource "aws_internet_gateway" "main" {
  vpc_id = aws_vpc.main.id
  tags   = { Name = "r3l-igw", Project = "r3l" }
}

############################
# Public subnets (NAT gateway lives here)
############################
resource "aws_route_table" "public" {
  vpc_id = aws_vpc.main.id

  route {
    cidr_block = "0.0.0.0/0"
    gateway_id = aws_internet_gateway.main.id
  }

  tags = { Name = "r3l-public-rt", Project = "r3l" }
}

resource "aws_subnet" "public_a" {
  vpc_id                  = aws_vpc.main.id
  cidr_block              = "10.0.1.0/24"
  availability_zone       = "${var.aws_region}a"
  map_public_ip_on_launch = true
  tags = { Name = "r3l-public-a", Project = "r3l" }
}

resource "aws_route_table_association" "public_a" {
  subnet_id      = aws_subnet.public_a.id
  route_table_id = aws_route_table.public.id
}

############################
# NAT Gateway (in public subnet, gives private subnets internet access)
############################
resource "aws_eip" "nat" {
  domain = "vpc"
  tags   = { Name = "r3l-nat-eip", Project = "r3l" }
}

resource "aws_nat_gateway" "main" {
  allocation_id = aws_eip.nat.id
  subnet_id     = aws_subnet.public_a.id
  tags          = { Name = "r3l-nat", Project = "r3l" }

  depends_on = [aws_internet_gateway.main]
}

############################
# Private subnets (App Runner + RDS)
############################
resource "aws_route_table" "private" {
  vpc_id = aws_vpc.main.id

  route {
    cidr_block     = "0.0.0.0/0"
    nat_gateway_id = aws_nat_gateway.main.id
  }

  tags = { Name = "r3l-private-rt", Project = "r3l" }
}

resource "aws_subnet" "private_a" {
  vpc_id            = aws_vpc.main.id
  cidr_block        = "10.0.10.0/24"
  availability_zone = "${var.aws_region}a"
  tags = { Name = "r3l-private-a", Project = "r3l" }
}

resource "aws_subnet" "private_b" {
  vpc_id            = aws_vpc.main.id
  cidr_block        = "10.0.11.0/24"
  availability_zone = "${var.aws_region}b"
  tags = { Name = "r3l-private-b", Project = "r3l" }
}

resource "aws_route_table_association" "private_a" {
  subnet_id      = aws_subnet.private_a.id
  route_table_id = aws_route_table.private.id
}

resource "aws_route_table_association" "private_b" {
  subnet_id      = aws_subnet.private_b.id
  route_table_id = aws_route_table.private.id
}

############################
# Security Groups
############################
resource "aws_security_group" "apprunner" {
  name_prefix = "r3l-apprunner-"
  vpc_id      = aws_vpc.main.id

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = { Project = "r3l" }
}

resource "aws_security_group" "db" {
  name_prefix = "r3l-db-"
  vpc_id      = aws_vpc.main.id

  # Only allow Postgres from App Runner security group
  ingress {
    from_port       = 5432
    to_port         = 5432
    protocol        = "tcp"
    security_groups = [aws_security_group.apprunner.id]
  }

  tags = { Project = "r3l" }
}

############################
# RDS: Postgres (db.t4g.micro) — private
############################
resource "aws_db_subnet_group" "main" {
  name       = "r3l-db-subnets"
  subnet_ids = [aws_subnet.private_a.id, aws_subnet.private_b.id]
  tags       = { Project = "r3l" }
}

resource "aws_db_instance" "postgres" {
  identifier     = "r3l-db"
  engine         = "postgres"
  engine_version = "16.4"
  instance_class = "db.t4g.micro"

  allocated_storage     = 20
  max_allocated_storage = 50
  storage_type          = "gp3"

  db_name  = "r3l"
  username = "r3l"
  password = var.db_password

  db_subnet_group_name   = aws_db_subnet_group.main.name
  vpc_security_group_ids = [aws_security_group.db.id]

  publicly_accessible = false
  skip_final_snapshot = true
  multi_az            = false

  tags = { Project = "r3l" }
}

############################
# ECR Repository
############################
resource "aws_ecr_repository" "r3l_app" {
  name                 = "r3l-app"
  image_tag_mutability = "MUTABLE"
  force_delete         = true

  image_scanning_configuration {
    scan_on_push = false
  }

  tags = { Project = "r3l" }
}

resource "aws_ecr_lifecycle_policy" "r3l_app" {
  repository = aws_ecr_repository.r3l_app.name
  policy = jsonencode({
    rules = [{
      rulePriority = 1
      description  = "Keep last 5 images"
      selection = {
        tagStatus   = "any"
        countType   = "imageCountMoreThan"
        countNumber = 5
      }
      action = { type = "expire" }
    }]
  })
}

############################
# IAM: App Runner → ECR access
############################
resource "aws_iam_role" "apprunner_ecr" {
  name = "r3l-app-apprunner-ecr"
  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Action    = "sts:AssumeRole"
      Effect    = "Allow"
      Principal = { Service = "build.apprunner.amazonaws.com" }
    }]
  })
}

resource "aws_iam_role_policy_attachment" "apprunner_ecr" {
  role       = aws_iam_role.apprunner_ecr.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AWSAppRunnerServicePolicyForECRAccess"
}

############################
# S3: Content Storage
############################
resource "aws_s3_bucket" "content" {
  bucket_prefix = "r3l-content-"
  force_destroy = true
  tags          = { Project = "r3l" }
}

resource "aws_s3_bucket_lifecycle_configuration" "content" {
  bucket = aws_s3_bucket.content.id

  rule {
    id     = "transition-to-ia"
    status = "Enabled"

    transition {
      days          = 30
      storage_class = "STANDARD_IA"
    }
  }
}

resource "aws_s3_bucket_public_access_block" "content" {
  bucket                  = aws_s3_bucket.content.id
  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

############################
# IAM: App Runner instance role
############################
resource "aws_iam_role" "apprunner_instance" {
  name = "r3l-app-apprunner-instance"
  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Action    = "sts:AssumeRole"
      Effect    = "Allow"
      Principal = { Service = "tasks.apprunner.amazonaws.com" }
    }]
  })
}

resource "aws_iam_role_policy" "apprunner_s3" {
  name = "r3l-content-s3-access"
  role = aws_iam_role.apprunner_instance.id
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect   = "Allow"
      Action   = ["s3:GetObject", "s3:PutObject", "s3:DeleteObject", "s3:ListBucket"]
      Resource = [
        aws_s3_bucket.content.arn,
        "${aws_s3_bucket.content.arn}/*",
      ]
    }]
  })
}

############################
# App Runner: VPC Connector (routes traffic through private subnets → NAT → internet)
############################
resource "aws_apprunner_vpc_connector" "main" {
  vpc_connector_name = "r3l-vpc-connector"
  subnets            = [aws_subnet.private_a.id, aws_subnet.private_b.id]
  security_groups    = [aws_security_group.apprunner.id]
  tags               = { Project = "r3l" }
}

############################
# App Runner: Auto-scaling (single instance for prototype)
############################
resource "aws_apprunner_auto_scaling_configuration_version" "single" {
  auto_scaling_configuration_name = "r3l-app-single"
  max_concurrency = 100
  max_size        = 1
  min_size        = 1
}

############################
# App Runner: Service
############################
resource "aws_apprunner_service" "r3l_app" {
  service_name = "r3l-app"

  source_configuration {
    authentication_configuration {
      access_role_arn = aws_iam_role.apprunner_ecr.arn
    }
    image_repository {
      image_configuration {
        port = "3001"
        runtime_environment_variables = {
          TRUST_DIR      = "/data/trust"
          BIND_ADDR      = "0.0.0.0:3001"
          VERIFIER_BIN   = "/app/verifier"
          STATIC_DIR     = "/app/static"
          SOLANA_RPC_URL = var.solana_rpc_url
          PROGRAM_ID     = var.program_id
          DATABASE_URL      = "postgresql://${aws_db_instance.postgres.username}:${var.db_password}@${aws_db_instance.postgres.endpoint}/${aws_db_instance.postgres.db_name}"
          STORAGE_BACKEND   = "s3"
          S3_BUCKET         = aws_s3_bucket.content.id
          S3_PREFIX         = "content/"
          SMTP_HOST         = var.smtp_host
          SMTP_USER         = var.smtp_user
          SMTP_PASS         = var.smtp_pass
          SMTP_FROM         = var.smtp_from
        }
      }
      image_identifier      = "${aws_ecr_repository.r3l_app.repository_url}:latest"
      image_repository_type = "ECR"
    }
    auto_deployments_enabled = true
  }

  instance_configuration {
    cpu               = "1024"  # 1 vCPU
    memory            = "4096"  # 4 GB (MobileCLIP2-S0 model + torch)
    instance_role_arn = aws_iam_role.apprunner_instance.arn
  }

  # VPC egress — App Runner routes through private subnets → NAT gateway → internet
  # Can reach both RDS (private) and Solana RPC (public internet)
  network_configuration {
    egress_configuration {
      egress_type       = "VPC"
      vpc_connector_arn = aws_apprunner_vpc_connector.main.arn
    }
  }

  auto_scaling_configuration_arn = aws_apprunner_auto_scaling_configuration_version.single.arn

  health_check_configuration {
    protocol            = "HTTP"
    path                = "/api/health"
    interval            = 10
    timeout             = 5
    healthy_threshold   = 1
    unhealthy_threshold = 10
  }

  tags = { Project = "r3l" }
}

############################
# CloudFront Distribution
############################
resource "aws_cloudfront_distribution" "r3l_app" {
  enabled         = true
  comment         = "R3L Provenance App"
  price_class     = "PriceClass_100" # US + Europe only (cheapest)
  is_ipv6_enabled = true

  origin {
    domain_name = aws_apprunner_service.r3l_app.service_url
    origin_id   = "apprunner"

    custom_origin_config {
      http_port              = 80
      https_port             = 443
      origin_protocol_policy = "https-only"
      origin_ssl_protocols   = ["TLSv1.2"]
    }
  }

  # API routes: no caching, pass everything through
  ordered_cache_behavior {
    path_pattern     = "/api/*"
    allowed_methods  = ["DELETE", "GET", "HEAD", "OPTIONS", "PATCH", "POST", "PUT"]
    cached_methods   = ["GET", "HEAD"]
    target_origin_id = "apprunner"

    forwarded_values {
      query_string = true
      headers      = ["Accept", "Authorization", "Content-Type", "Origin", "Referer"]
      cookies {
        forward = "all"
      }
    }

    viewer_protocol_policy = "redirect-to-https"
    min_ttl                = 0
    default_ttl            = 0
    max_ttl                = 0
  }

  # Static assets: cache at edge
  default_cache_behavior {
    allowed_methods  = ["GET", "HEAD", "OPTIONS"]
    cached_methods   = ["GET", "HEAD"]
    target_origin_id = "apprunner"

    forwarded_values {
      query_string = false
      cookies {
        forward = "none"
      }
    }

    viewer_protocol_policy = "redirect-to-https"
    min_ttl                = 0
    default_ttl            = 3600    # 1 hour
    max_ttl                = 86400   # 1 day
    compress               = true
  }

  restrictions {
    geo_restriction {
      restriction_type = "none"
    }
  }

  # Free CloudFront HTTPS cert (*.cloudfront.net)
  viewer_certificate {
    cloudfront_default_certificate = true
  }

  tags = { Project = "r3l" }
}

############################
# Outputs
############################
output "ecr_repository_url" {
  value = aws_ecr_repository.r3l_app.repository_url
}

output "apprunner_url" {
  value = "https://${aws_apprunner_service.r3l_app.service_url}"
}

output "cloudfront_url" {
  value = "https://${aws_cloudfront_distribution.r3l_app.domain_name}"
}

output "cloudfront_distribution_id" {
  value = aws_cloudfront_distribution.r3l_app.id
}

output "rds_endpoint" {
  value = aws_db_instance.postgres.endpoint
}

output "pause_command" {
  value = "aws apprunner pause-service --service-arn ${aws_apprunner_service.r3l_app.arn}"
}

output "resume_command" {
  value = "aws apprunner resume-service --service-arn ${aws_apprunner_service.r3l_app.arn}"
}
