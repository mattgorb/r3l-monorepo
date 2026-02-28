import os
from pathlib import Path
from pydantic_settings import BaseSettings


class Settings(BaseSettings):
    trust_dir: str = "../../data/trust"
    prover_dir: str = "../prover"
    verifier_bin: str = "../verifier/target/release/verifier"
    solana_rpc_url: str = "http://127.0.0.1:8899"
    solana_keypair_path: str = str(
        Path.home() / ".config" / "solana" / "id.json"
    )
    program_id: str = "63jq6M3t5NafYWcADqLDCLnhd5qPfEmCUcaA9iWh5YWz"
    static_dir: str = "../web/dist"
    bind_addr: str = "0.0.0.0:3001"
    prover_mock: str = "true"
    public_url: str = "http://localhost:3001"
    database_url: str = "postgresql://postgres:postgres@localhost:5432/r3l"

    # Storage
    storage_backend: str = "local"           # "local" or "s3"
    storage_dir: str = "../../data/storage"  # for local backend
    s3_bucket: str = ""                      # for s3 backend
    s3_prefix: str = "content/"              # key prefix in bucket

    # SMTP (optional — dev mode if not set)
    smtp_host: str = ""
    smtp_user: str = ""
    smtp_pass: str = ""
    smtp_from: str = ""

    model_config = {"env_file": "../../.env", "extra": "ignore"}
