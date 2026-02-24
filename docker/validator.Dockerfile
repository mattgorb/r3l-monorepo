FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
  curl ca-certificates bzip2 \
  && sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)" \
  && rm -rf /var/lib/apt/lists/*

ENV PATH="/root/.local/share/solana/install/active_release/bin:$PATH"

COPY docker/validator-entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

EXPOSE 8899 8900
ENTRYPOINT ["/entrypoint.sh"]
