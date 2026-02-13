import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, SystemProgram, ComputeBudgetProgram } from "@solana/web3.js";
import { expect } from "chai";

const ATTESTATION_SEED = Buffer.from("attestation");

describe("provenance_attestation", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.ProvenanceAttestation as Program;

  // Sample content hash (SHA-256 of a test file)
  const contentHash = Buffer.alloc(32);
  contentHash.write("deadbeef", 0, "hex");
  contentHash.fill(0xab, 4);

  const [attestationPda] = PublicKey.findProgramAddressSync(
    [ATTESTATION_SEED, contentHash],
    program.programId
  );

  it("submits a proof and stores an attestation", async () => {
    // Mock proof data (proof verification is TODO, so empty bytes work)
    const proof = Buffer.alloc(0);
    const publicInputs = Buffer.alloc(0);

    const computeBudgetIx = ComputeBudgetProgram.setComputeUnitLimit({
      units: 400_000,
    });

    const tx = await (program.methods as any)
      .submitProof(
        proof,
        publicInputs,
        Array.from(contentHash),
        true,                          // has_c2pa
        "official",                     // trust_list_match
        "Trusted",                      // validation_state
        "http://cv.iptc.org/newscodes/digitalsourcetype/trainedAlgorithmicMedia",
        "OpenAI",                       // issuer
        "Truepic Lens CLI in Sora",    // common_name
        "GPT-4o",                       // software_agent
        "2024-12-01T00:00:00Z"         // signing_time
      )
      .accounts({
        attestation: attestationPda,
        submitter: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .preInstructions([computeBudgetIx])
      .rpc();

    console.log("    tx:", tx);

    // Fetch and verify — use fallback for both camelCase and snake_case field names
    const attestation: any = await (program.account as any).attestation.fetch(attestationPda);
    console.log("    fields:", Object.keys(attestation));

    expect(Buffer.from(attestation.contentHash).toString("hex")).to.equal(
      contentHash.toString("hex")
    );
    expect(attestation.hasC2Pa).to.be.true;
    expect(attestation.trustListMatch).to.equal("official");
    expect(attestation.validationState).to.equal("Trusted");
    expect(attestation.issuer).to.equal("OpenAI");
    expect(attestation.commonName).to.equal("Truepic Lens CLI in Sora");
    expect(attestation.softwareAgent).to.equal("GPT-4o");
    expect(attestation.signingTime).to.equal("2024-12-01T00:00:00Z");
    expect(attestation.submittedBy.toBase58()).to.equal(
      provider.wallet.publicKey.toBase58()
    );
    expect(attestation.timestamp.toNumber()).to.be.greaterThan(0);
  });

  it("rejects duplicate attestation for same content hash", async () => {
    const proof = Buffer.alloc(0);
    const publicInputs = Buffer.alloc(0);

    try {
      await (program.methods as any)
        .submitProof(
          proof,
          publicInputs,
          Array.from(contentHash),
          true,
          "official",
          "Trusted",
          "",
          "OpenAI",
          "Test",
          "Test",
          ""
        )
        .accounts({
          attestation: attestationPda,
          submitter: provider.wallet.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
      expect.fail("should have thrown");
    } catch (e: any) {
      // Account already initialized — Anchor rejects init on existing account
      expect(e.toString()).to.include("already in use");
    }
  });

  it("rejects strings that exceed max length", async () => {
    const badHash = Buffer.alloc(32, 0xff);
    const [badPda] = PublicKey.findProgramAddressSync(
      [ATTESTATION_SEED, badHash],
      program.programId
    );

    const longString = "x".repeat(129);

    try {
      await (program.methods as any)
        .submitProof(
          Buffer.alloc(0),
          Buffer.alloc(0),
          Array.from(badHash),
          false,
          longString,  // exceeds MAX_STRING_LEN
          "Valid",
          "",
          "",
          "",
          "",
          ""
        )
        .accounts({
          attestation: badPda,
          submitter: provider.wallet.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
      expect.fail("should have thrown");
    } catch (e: any) {
      expect(e.toString()).to.include("StringTooLong");
    }
  });
});
