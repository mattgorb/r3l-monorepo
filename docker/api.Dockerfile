# ---- build ----
FROM rust:1-bookworm AS build

RUN apt-get update && apt-get install -y --no-install-recommends \
  pkg-config \
  libssl-dev \
  ca-certificates \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace

# Copy verifier (path dependency) and api
COPY services/verifier/ services/verifier/
COPY services/api/ services/api/

WORKDIR /workspace/services/api
RUN cargo build --release

# ---- frontend ----
FROM node:22-slim AS frontend

WORKDIR /workspace/services/web
COPY services/web/package.json services/web/package-lock.json ./
RUN npm ci
COPY services/web/ .
RUN npm run build

# ---- runtime ----
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
  ca-certificates \
  libssl3 \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=build /workspace/services/api/target/release/api /app/api
COPY --from=frontend /workspace/services/web/dist /app/static

ENV TRUST_DIR=/data/trust
ENV BIND_ADDR=0.0.0.0:3001

EXPOSE 3001
ENTRYPOINT ["/app/api"]
