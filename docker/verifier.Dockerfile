# ---- build ----
FROM rust:1-bookworm AS build

RUN apt-get update && apt-get install -y --no-install-recommends \
  pkg-config \
  libssl-dev \
  ca-certificates \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace

# Copy only manifests first for better caching
COPY services/verifier/Cargo.toml services/verifier/Cargo.toml
COPY services/verifier/src services/verifier/src

WORKDIR /workspace/services/verifier
RUN cargo build --release

# ---- runtime ----
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
  ca-certificates \
  libssl3 \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=build /workspace/services/verifier/target/release/verifier /app/verifier

# Trust lists are mounted at runtime via volumes (not embedded at compile time)
# Mount data/trust/ to /data/trust/ with official/ and curated/ subdirs

ENTRYPOINT ["/app/verifier"]
