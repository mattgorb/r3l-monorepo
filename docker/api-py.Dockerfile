# ---- build verifier (Rust) ----
FROM rust:1-bookworm AS verifier-build

RUN apt-get update && apt-get install -y --no-install-recommends \
  pkg-config libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace
COPY services/verifier/ services/verifier/
WORKDIR /workspace/services/verifier
RUN cargo build --release

# ---- build frontend ----
FROM node:22-slim AS frontend

WORKDIR /workspace/services/web
COPY services/web/package.json services/web/package-lock.json ./
RUN npm ci
COPY services/web/ .
RUN npm run build

# ---- Python runtime ----
FROM python:3.12-slim-bookworm AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
  ca-certificates libssl3 ffmpeg g++ && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY services/api-py/requirements.txt .
# Install PyTorch CPU-only first (smaller image), then the rest
RUN pip install --no-cache-dir torch torchvision --index-url https://download.pytorch.org/whl/cpu && \
    pip install --no-cache-dir -r requirements.txt && \
    apt-get purge -y --auto-remove g++

# Pre-download MobileCLIP2-S0 model weights so there's no HuggingFace download at runtime
RUN python -c "import open_clip; open_clip.create_model_and_transforms('MobileCLIP2-S0', pretrained='dfndr2b')"

COPY services/api-py/ /app/
COPY --from=verifier-build /workspace/services/verifier/target/release/verifier /app/verifier
COPY --from=frontend /workspace/services/web/dist /app/static
COPY data/trust /data/trust

# Solana keypair (copied by deploy.sh, optional for local builds)
COPY docker/.solana-keypair.jso[n] /app/solana-keypair.json

ENV TRUST_DIR=/data/trust
ENV VERIFIER_BIN=/app/verifier
ENV STATIC_DIR=/app/static
ENV SOLANA_KEYPAIR_PATH=/app/solana-keypair.json

EXPOSE 3001
CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "3001"]
