import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, SystemProgram, ComputeBudgetProgram } from "@solana/web3.js";
import * as fs from "fs";
import * as path from "path";

// Load IDL from anchor build output
const IDL_PATH = path.join(__dirname, "..", "target", "idl", "provenance_attestation.json");

const ATTESTATION_SEED = Buffer.from("attestation");

interface ProofOutput {
  proof: number[];        // Groth16 proof bytes
  public_values: number[]; // SP1 public values bytes
  public_outputs: {
    content_hash: number[];
    has_c2pa: boolean;
    trust_list_match: string;
    validation_state: string;
    digital_source_type: string;
    issuer: string;
    common_name: string;
    software_agent: string;
    signing_time: string;
  };
}

async function main() {
  const args = process.argv.slice(2);

  if (args.includes("--help") || args.length === 0) {
    console.log(`Usage:
  Submit attestation with proof:
    npx ts-node scripts/submit.ts --proof <proof.json>

  Query attestation by content hash:
    npx ts-node scripts/submit.ts --query <content_hash_hex>

Environment:
  ANCHOR_PROVIDER_URL  Solana RPC URL (default: http://localhost:8899)
  ANCHOR_WALLET        Wallet keypair path (default: ~/.config/solana/id.json)`);
    process.exit(0);
  }

  // Set up Anchor provider
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  // Load IDL
  if (!fs.existsSync(IDL_PATH)) {
    console.error(`IDL not found at ${IDL_PATH}. Run 'anchor build' first.`);
    process.exit(1);
  }
  const idl = JSON.parse(fs.readFileSync(IDL_PATH, "utf8"));
  const programId = new PublicKey(idl.address);
  const program = new Program(idl, provider);

  if (args.includes("--query")) {
    const hashIdx = args.indexOf("--query") + 1;
    if (hashIdx >= args.length) {
      console.error("Missing content hash argument");
      process.exit(1);
    }
    await queryAttestation(program, args[hashIdx]);
  } else if (args.includes("--proof")) {
    const proofIdx = args.indexOf("--proof") + 1;
    if (proofIdx >= args.length) {
      console.error("Missing proof file argument");
      process.exit(1);
    }
    await submitProof(program, provider, args[proofIdx]);
  } else {
    console.error("Unknown command. Use --proof or --query.");
    process.exit(1);
  }
}

async function submitProof(
  program: Program,
  provider: anchor.AnchorProvider,
  proofPath: string
) {
  const proofData: ProofOutput = JSON.parse(fs.readFileSync(proofPath, "utf8"));
  const { public_outputs } = proofData;

  const contentHash = Buffer.from(public_outputs.content_hash);
  console.log(`Content hash: ${contentHash.toString("hex")}`);

  // Derive PDA
  const [attestationPda, bump] = PublicKey.findProgramAddressSync(
    [ATTESTATION_SEED, contentHash],
    program.programId
  );
  console.log(`Attestation PDA: ${attestationPda.toBase58()}`);

  // Check if attestation already exists
  const existing = await provider.connection.getAccountInfo(attestationPda);
  if (existing) {
    console.log("Attestation already exists for this content hash.");
    process.exit(0);
  }

  // Build transaction with compute budget increase (280K CU for proof verification)
  const computeBudgetIx = ComputeBudgetProgram.setComputeUnitLimit({
    units: 400_000,
  });

  const tx = await (program.methods as any)
    .submitProof(
      Buffer.from(proofData.proof),
      Buffer.from(proofData.public_values),
      Array.from(contentHash),
      public_outputs.has_c2pa,
      public_outputs.trust_list_match,
      public_outputs.validation_state,
      public_outputs.digital_source_type,
      public_outputs.issuer,
      public_outputs.common_name,
      public_outputs.software_agent,
      public_outputs.signing_time
    )
    .accounts({
      attestation: attestationPda,
      submitter: provider.wallet.publicKey,
      systemProgram: SystemProgram.programId,
    })
    .preInstructions([computeBudgetIx])
    .rpc();

  console.log(`Transaction signature: ${tx}`);
  console.log(`Attestation stored at PDA: ${attestationPda.toBase58()}`);

  // Verify by reading back
  const attestation = await (program.account as any).attestation.fetch(attestationPda);
  console.log("\nStored attestation:");
  console.log(`  has_c2pa: ${attestation.hasC2pa}`);
  console.log(`  trust_list_match: ${attestation.trustListMatch}`);
  console.log(`  validation_state: ${attestation.validationState}`);
  console.log(`  issuer: ${attestation.issuer}`);
  console.log(`  common_name: ${attestation.commonName}`);
  console.log(`  software_agent: ${attestation.softwareAgent}`);
  console.log(`  submitted_by: ${attestation.submittedBy.toBase58()}`);
}

async function queryAttestation(program: Program, contentHashHex: string) {
  // Strip 0x prefix if present
  const hex = contentHashHex.startsWith("0x") ? contentHashHex.slice(2) : contentHashHex;
  const contentHash = Buffer.from(hex, "hex");

  if (contentHash.length !== 32) {
    console.error(`Invalid content hash: expected 32 bytes, got ${contentHash.length}`);
    process.exit(1);
  }

  const [attestationPda] = PublicKey.findProgramAddressSync(
    [ATTESTATION_SEED, contentHash],
    program.programId
  );

  console.log(`Looking up PDA: ${attestationPda.toBase58()}`);

  try {
    const attestation = await (program.account as any).attestation.fetch(attestationPda);
    console.log("\nAttestation found:");
    console.log(JSON.stringify({
      content_hash: Buffer.from(attestation.contentHash).toString("hex"),
      has_c2pa: attestation.hasC2pa,
      trust_list_match: attestation.trustListMatch,
      validation_state: attestation.validationState,
      digital_source_type: attestation.digitalSourceType,
      issuer: attestation.issuer,
      common_name: attestation.commonName,
      software_agent: attestation.softwareAgent,
      signing_time: attestation.signingTime,
      submitted_by: attestation.submittedBy.toBase58(),
      timestamp: attestation.timestamp.toNumber(),
    }, null, 2));
  } catch (e: any) {
    if (e.message?.includes("Account does not exist")) {
      console.log("No attestation found for this content hash.");
    } else {
      throw e;
    }
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
