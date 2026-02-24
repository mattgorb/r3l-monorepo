use std::fs;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};

/// R3L Edge Node CLI — verify files locally, attest on-chain.
#[derive(Parser)]
#[command(name = "r3l-edge", version)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Register edge node and get API key
    Register {
        /// Node name
        #[arg(long)]
        name: Option<String>,
        /// Path to Ed25519 keypair JSON
        #[arg(long, default_value = "edge-keypair.json")]
        keypair: PathBuf,
        /// API base URL
        #[arg(long, env = "R3L_API_URL", default_value = "http://localhost:3001")]
        api: String,
    },
    /// Verify a file locally and submit attestation
    Attest {
        /// Path to media file
        file: PathBuf,
        /// Path to Ed25519 keypair JSON
        #[arg(long, default_value = "edge-keypair.json")]
        keypair: PathBuf,
        /// API base URL
        #[arg(long, env = "R3L_API_URL", default_value = "http://localhost:3001")]
        api: String,
        /// API key
        #[arg(long, env = "R3L_API_KEY")]
        api_key: String,
        /// Path to verifier binary
        #[arg(long, default_value = "verifier")]
        verifier: String,
        /// Path to trust directory
        #[arg(long, default_value = "data/trust")]
        trust_dir: String,
    },
    /// Hash a file (SHA-256)
    Hash {
        /// Path to file
        file: PathBuf,
    },
    /// Query structured trust verdict
    Query {
        /// Content hash (hex)
        hash: String,
        /// API base URL
        #[arg(long, env = "R3L_API_URL", default_value = "http://localhost:3001")]
        api: String,
    },
    /// Look up raw attestation data
    Lookup {
        /// Content hash (hex)
        hash: String,
        /// API base URL
        #[arg(long, env = "R3L_API_URL", default_value = "http://localhost:3001")]
        api: String,
    },
}

// ── Keypair helpers ──────────────────────────────────────────────

fn load_keypair(path: &PathBuf) -> Result<SigningKey> {
    let data = fs::read_to_string(path)
        .with_context(|| format!("reading keypair: {}", path.display()))?;
    let bytes: Vec<u8> = serde_json::from_str::<Vec<u8>>(&data)
        .context("parsing keypair JSON")?;
    if bytes.len() < 64 {
        bail!("keypair must be 64 bytes, got {}", bytes.len());
    }
    let secret: [u8; 32] = bytes[..32].try_into()?;
    Ok(SigningKey::from_bytes(&secret))
}

fn generate_keypair(path: &PathBuf) -> Result<SigningKey> {
    let mut rng = rand::thread_rng();
    let key = SigningKey::generate(&mut rng);
    let mut full = Vec::with_capacity(64);
    full.extend_from_slice(&key.to_bytes());
    full.extend_from_slice(key.verifying_key().as_bytes());
    let json = serde_json::to_string(&full)?;
    fs::write(path, &json)
        .with_context(|| format!("writing keypair: {}", path.display()))?;
    eprintln!("Generated keypair: {}", path.display());
    Ok(key)
}

fn pubkey_b58(key: &SigningKey) -> String {
    bs58::encode(key.verifying_key().as_bytes()).into_string()
}

fn sign_b58(key: &SigningKey, msg: &str) -> String {
    let sig = key.sign(msg.as_bytes());
    bs58::encode(sig.to_bytes()).into_string()
}

// ── HTTP helpers ─────────────────────────────────────────────────

fn post_json(url: &str, body: &serde_json::Value, headers: &[(&str, &str)]) -> Result<serde_json::Value> {
    let client = reqwest::blocking::Client::new();
    let mut req = client.post(url).json(body);
    for (k, v) in headers {
        req = req.header(*k, *v);
    }
    let resp = req.send().context("HTTP POST failed")?;
    let status = resp.status();
    let text = resp.text().context("reading response body")?;
    if !status.is_success() {
        bail!("HTTP {}: {}", status, text);
    }
    serde_json::from_str(&text).context("parsing response JSON")
}

fn get_json(url: &str) -> Result<serde_json::Value> {
    let resp = reqwest::blocking::get(url).context("HTTP GET failed")?;
    let status = resp.status();
    let text = resp.text().context("reading response body")?;
    if !status.is_success() {
        bail!("HTTP {}: {}", status, text);
    }
    serde_json::from_str(&text).context("parsing response JSON")
}

// ── Hash ─────────────────────────────────────────────────────────

fn hash_file(path: &PathBuf) -> Result<String> {
    let data = fs::read(path)
        .with_context(|| format!("reading file: {}", path.display()))?;
    let hash = Sha256::digest(&data);
    Ok(hex::encode(hash))
}

// ── Commands ─────────────────────────────────────────────────────

fn cmd_register(name: Option<String>, keypair: PathBuf, api: String) -> Result<()> {
    let key = if keypair.exists() {
        eprintln!("Using existing keypair: {}", keypair.display());
        load_keypair(&keypair)?
    } else {
        generate_keypair(&keypair)?
    };

    let pubkey = pubkey_b58(&key);
    let sig = sign_b58(&key, "R3L: register");

    let body = serde_json::json!({
        "pubkey": pubkey,
        "message": "R3L: register",
        "signature": sig,
        "name": name.unwrap_or_else(|| format!("edge-{}", &pubkey[..8])),
    });

    let resp = post_json(&format!("{api}/api/edge/register"), &body, &[])?;

    println!("\nRegistered successfully!");
    println!("  Pubkey:  {}", resp["pubkey"].as_str().unwrap_or(""));
    println!("  Name:    {}", resp["name"].as_str().unwrap_or(""));
    println!("  API Key: {}", resp["api_key"].as_str().unwrap_or(""));
    println!("\nSave your API key:");
    println!("  export R3L_API_KEY={}", resp["api_key"].as_str().unwrap_or(""));
    Ok(())
}

fn cmd_attest(
    file: PathBuf,
    keypair: PathBuf,
    api: String,
    api_key: String,
    verifier: String,
    trust_dir: String,
) -> Result<()> {
    if !file.exists() {
        bail!("File not found: {}", file.display());
    }

    // 1. Run verifier
    eprintln!("Verifying: {}", file.display());
    let mut cmd = Command::new(&verifier);
    if !trust_dir.is_empty() && std::path::Path::new(&trust_dir).is_dir() {
        cmd.arg("--trust-dir").arg(&trust_dir);
    }
    cmd.arg(&file);

    let output = cmd.output().with_context(|| format!("running verifier: {verifier}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Verifier failed: {stderr}");
    }

    let verify_output: serde_json::Value = serde_json::from_slice(&output.stdout)
        .context("parsing verifier JSON output")?;

    let content_hash = verify_output["content_hash"]
        .as_str()
        .context("no content_hash in verifier output")?;

    eprintln!("Content hash: {content_hash}");
    eprintln!("C2PA: {}", verify_output["has_c2pa"].as_bool().unwrap_or(false));

    // 1b. Compute TLSH hash for similarity search
    let file_bytes = fs::read(&file)
        .with_context(|| format!("reading file for TLSH: {}", file.display()))?;
    let tlsh_hash = {
        let mut builder = tlsh2::TlshDefaultBuilder::new();
        builder.update(&file_bytes);
        builder.build()
            .map(|h| h.hash().to_string())
            .unwrap_or_default()
    };
    if !tlsh_hash.is_empty() {
        eprintln!("TLSH: {tlsh_hash}");
    }

    // 2. Build attestation body
    let mut body = serde_json::json!({
        "content_hash": content_hash,
        "has_c2pa": verify_output["has_c2pa"].as_bool().unwrap_or(false),
        "trust_list_match": verify_output["trust_list_match"].as_str().unwrap_or(""),
        "validation_state": verify_output["validation_state"].as_str().unwrap_or(""),
        "digital_source_type": verify_output["digital_source_type"].as_str().unwrap_or(""),
        "issuer": verify_output["issuer"].as_str().unwrap_or(""),
        "common_name": verify_output["common_name"].as_str().unwrap_or(""),
        "software_agent": verify_output["software_agent"].as_str().unwrap_or(""),
        "signing_time": verify_output["signing_time"].as_str().unwrap_or(""),
    });

    // 2b. Add TLSH hash if computed
    if !tlsh_hash.is_empty() {
        body["tlsh_hash"] = serde_json::Value::String(tlsh_hash);
    }

    // 3. Sign wallet message if keypair exists
    if keypair.exists() {
        if let Ok(key) = load_keypair(&keypair) {
            let msg = format!("R3L: attest {content_hash}");
            let wallet_sig = sign_b58(&key, &msg);
            body["wallet_signature"] = serde_json::Value::String(wallet_sig);
            eprintln!("Wallet signature: included");
        }
    }

    // 4. Submit
    eprintln!("Submitting attestation...");
    let resp = post_json(
        &format!("{api}/api/edge/attest"),
        &body,
        &[("X-API-Key", &api_key)],
    )?;

    if resp.get("existing").and_then(|v| v.as_bool()).unwrap_or(false) {
        println!("\nAttestation already exists:");
    } else {
        println!("\nAttestation created:");
    }
    println!("  Content hash: {}", resp["content_hash"].as_str().unwrap_or(""));
    println!("  PDA:          {}", resp["attestation_pda"].as_str().unwrap_or(""));
    if let Some(sig) = resp["signature"].as_str() {
        println!("  Tx signature: {sig}");
    }
    if let Some(w) = resp["wallet_pubkey"].as_str() {
        println!("  Wallet:       {w}");
    }

    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Cmd::Register { name, keypair, api } => cmd_register(name, keypair, api),
        Cmd::Attest { file, keypair, api, api_key, verifier, trust_dir } => {
            cmd_attest(file, keypair, api, api_key, verifier, trust_dir)
        }
        Cmd::Hash { file } => {
            let hash = hash_file(&file)?;
            println!("{hash}  {}", file.display());
            Ok(())
        }
        Cmd::Query { hash, api } => {
            let resp = get_json(&format!("{api}/api/v1/query/{hash}"))?;
            println!("{}", serde_json::to_string_pretty(&resp)?);
            Ok(())
        }
        Cmd::Lookup { hash, api } => {
            let resp = get_json(&format!("{api}/api/attestation/{hash}"))?;
            println!("{}", serde_json::to_string_pretty(&resp)?);
            Ok(())
        }
    }
}
