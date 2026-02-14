use anyhow::Result;
use clap::Parser;
use prover_script::jumbf_extract;
use prover_shared::PublicOutputs;
use sp1_prover::components::CpuProverComponents;
use sp1_sdk::{include_elf, HashableKey, Prover, ProverClient, SP1ProofMode, SP1Stdin};

const ELF: &[u8] = include_elf!("provenance-program");

#[derive(Parser)]
#[command(about = "Generate a Groth16 proof of C2PA verification")]
struct Args {
    /// Path to the media file to verify
    #[arg(long)]
    media: String,

    /// Trust directory containing official/ and curated/ PEM subdirectories
    #[arg(long, env = "TRUST_DIR", default_value = "/data/trust")]
    trust_dir: String,

    /// Output path for the proof file
    #[arg(long, default_value = "proof.bin")]
    output: String,

    /// Use mock prover for testing (no real proof generation)
    #[arg(long)]
    mock: bool,

    /// Write JSON sidecar with proof and public_values hex
    #[arg(long)]
    json_out: Option<String>,
}

fn main() -> Result<()> {
    sp1_sdk::utils::setup_logger();
    let args = Args::parse();

    // Extract all cryptographic evidence from the media file
    let evidence = jumbf_extract::extract_crypto_evidence(&args.media, &args.trust_dir)?;

    println!("Asset hash: {}", hex::encode(evidence.asset_hash));
    println!("Has manifest: {}", evidence.has_manifest);
    println!("COSE signature: {} bytes", evidence.cose_sign1_bytes.len());
    println!("Cert chain: {} cert(s)", evidence.cert_chain_der.len());
    println!("Claim CBOR: {} bytes", evidence.claim_cbor.len());

    // Write CryptoEvidence to SP1 stdin — guest derives everything from this
    let mut stdin = SP1Stdin::new();
    stdin.write(&evidence);

    // Create prover client and run
    if args.mock {
        let client = ProverClient::builder().mock().build();
        run_prover(client, ELF, stdin, &args.output, args.json_out.as_deref())?;
    } else if std::env::var("SP1_PROVER").unwrap_or_default() == "cuda" {
        println!("Using CUDA GPU prover (set via SP1_PROVER=cuda)");
        let client = ProverClient::builder().cuda().build();
        run_prover(client, ELF, stdin, &args.output, args.json_out.as_deref())?;
    } else {
        println!("Using CPU prover (set SP1_PROVER=cuda for GPU)");
        let client = ProverClient::builder().cpu().build();
        run_prover(client, ELF, stdin, &args.output, args.json_out.as_deref())?;
    };

    Ok(())
}

fn run_prover(
    client: impl Prover<CpuProverComponents>,
    elf: &[u8],
    stdin: SP1Stdin,
    output_path: &str,
    json_out: Option<&str>,
) -> Result<()> {
    let (pk, vk) = client.setup(elf);
    println!("vkey hash: {}", vk.bytes32());

    // Execute first to check correctness
    let (mut public_values, report) = client.execute(elf, &stdin)?;
    println!("executed in {} cycles", report.total_instruction_count());

    let outputs: PublicOutputs = public_values.read();
    println!("--- Public Outputs ---");
    println!("content_hash: {}", hex::encode(outputs.content_hash));
    println!("has_c2pa: {}", outputs.has_c2pa);
    println!("trust_list_match: {}", outputs.trust_list_match);
    println!("validation_state: {}", outputs.validation_state);
    println!("issuer: {}", outputs.issuer);
    println!("common_name: {}", outputs.common_name);
    println!("software_agent: {}", outputs.software_agent);
    println!("digital_source_type: {}", outputs.digital_source_type);
    println!("signing_time: {}", outputs.signing_time);
    println!("cert_fingerprint: {}", outputs.cert_fingerprint);

    // Generate Groth16 proof
    println!("generating Groth16 proof...");
    let proof = client.prove(&pk, &stdin, SP1ProofMode::Groth16)?;

    // Verify locally
    client
        .verify(&proof, &vk)
        .expect("proof verification failed");
    println!("proof verified locally");

    // Save proof
    proof.save(output_path)?;
    println!("proof saved to {}", output_path);

    let proof_bytes = proof.bytes();
    let public_values_bytes = proof.public_values.as_slice();
    println!("proof bytes: {} bytes", proof_bytes.len());
    println!("public values: {} bytes", public_values_bytes.len());

    // Write JSON sidecar if requested
    if let Some(json_path) = json_out {
        let sidecar = serde_json::json!({
            "proof": hex::encode(&proof_bytes),
            "public_values": hex::encode(public_values_bytes),
        });
        std::fs::write(json_path, serde_json::to_string_pretty(&sidecar)?)?;
        println!("JSON sidecar written to {}", json_path);
    }

    Ok(())
}
