## Quick Start

```bash
# local (requires Rust toolchain)
cargo run -- /path/to/image.png

# docker — production image
docker compose run --rm --build verifier /data/samples/chatgpt.png

# docker — dev image (mounts workspace, has c2patool)
docker compose run --rm --build verifier-dev
```

## Test Files

```bash
docker compose run --rm --build verifier /data/samples/Firefly_Gemini.png
docker compose run --rm --build verifier /data/samples/park-bench-sunset-c2pa-max-vmh.mp4
docker compose run --rm --build verifier /data/fixtures/c2pa-public-testfiles/legacy/1.4/image/jpeg/adobe-20220124-C.jpg
docker compose run --rm --build verifier /data/fixtures/c2pa-public-testfiles/legacy/1.4/image/jpeg/adobe-20220124-XCI.jpg
docker compose run --rm --build verifier /data/fixtures/c2pa-public-testfiles/legacy/1.4/image/jpeg/adobe-20220124-CII.jpg
```

## Verifier Output

The verifier prints a single JSON object to stdout with these fields:

| Field | Type | Description |
|---|---|---|
| `path` | string | Input file path |
| `has_c2pa` | bool | Whether C2PA manifest data was found |
| `trust_list_match` | string? | `"official"`, `"curated"`, or `"untrusted"` |
| `validation_state` | string? | `"Trusted"`, `"Valid"`, or `"Invalid"` |
| `validation_error_count` | int? | Number of validation status entries |
| `validation_codes` | string[]? | Actual validation status codes (e.g. `signingCredential.untrusted`) |
| `title` | string? | Asset title from the manifest |
| `format` | string? | Media type (e.g. `image/png`, `video/mp4`) |
| `digital_source_type` | string? | IPTC digital source type URI |
| `claim_generator` | string? | Tool that generated the claim |
| `software_agent` | string? | Software that created the content |
| `issuer` | string? | Certificate issuer organization |
| `common_name` | string? | Certificate common name |
| `signing_time` | string? | ISO timestamp of signature |
| `sig_algorithm` | string? | Signature algorithm (e.g. `Es256`) |
| `actions` | json? | Raw c2pa actions array |
| `ingredients` | json? | Raw ingredients array |
| `manifest_store` | json? | Full manifest store dump |
| `error` | string? | Error message if verification failed |

Exit code is 0 on success, 1 if there's an error.

## Trust Lists

Trust anchors are PEM certificate files stored in two directories:

```
data/trust/
  official/     # C2PA official trust list (Google, Adobe, DigiCert, etc.)
  curated/      # Your own trusted signers (OpenAI, Firefly, test certs, etc.)
```

The verifier tries **official first**, then **curated**. If the signer matches neither, `trust_list_match` is `"untrusted"`.

Each `.pem` file in these directories can contain one or more certificates. Files are loaded alphabetically and concatenated at runtime.

### Adding a trust record

```bash
# From a PEM file directly:
./scripts/add-trust.sh curated my-org /path/to/certs.pem

# Extract certs from a signed image (requires c2patool):
./scripts/add-trust.sh curated adobe-firefly /path/to/signed-image.png
```

### Extracting certs manually with c2patool

```bash
docker compose build verifier-dev
docker compose run --rm verifier-dev c2patool /workspace/data/samples/chatgpt.png --certs
```

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `TRUST_DIR` | `/data/trust` | Base directory for trust list PEM files |

## Project Structure

```
services/verifier/     Rust CLI — reads C2PA manifests, validates trust, outputs JSON
docker/
  verifier.Dockerfile  Production multi-stage build
  dev.Dockerfile       Dev image with c2patool
scripts/
  add-trust.sh         Add a PEM or extract certs from a signed image
data/
  trust/official/      Official C2PA trust anchors (one .pem per org)
  trust/curated/       User-curated trust anchors
  samples/             Test images
```






cargo run --bin prove -- --file ../../data/samples/chatgpt.png




# From services/prover/script/ directory\
TRUST_DIR=../../data/trust cargo run -- ../../data/samples/chatgpt.png 2>/dev/null > ../prover/script/verify.json


cd ../prover/script
cargo run --bin prove -- --file verify.json --media ../../../data/samples/chatgpt.png
