import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, SystemProgram, ComputeBudgetProgram } from "@solana/web3.js";
import { expect } from "chai";

const ATTESTATION_SEED = Buffer.from("attestation");

/**
 * Encode a PublicOutputs struct in bincode 1.x format.
 * Must match the on-chain parser in lib.rs and the SP1 guest's commit format.
 *
 * Layout:
 * - [u8; 32]: raw bytes (content_hash)
 * - bool: 1 byte (has_c2pa)
 * - 8x String: u64 LE length prefix + UTF-8 bytes
 */
function encodeBincodePublicOutputs(outputs: {
  contentHash: Buffer;
  hasC2pa: boolean;
  trustListMatch: string;
  validationState: string;
  digitalSourceType: string;
  issuer: string;
  commonName: string;
  softwareAgent: string;
  signingTime: string;
  certFingerprint: string;
}): Buffer {
  const parts: Buffer[] = [];

  // content_hash: [u8; 32] - raw bytes
  parts.push(outputs.contentHash);

  // has_c2pa: bool - 1 byte
  parts.push(Buffer.from([outputs.hasC2pa ? 1 : 0]));

  // 8 String fields: u64 LE length + UTF-8 bytes
  for (const s of [
    outputs.trustListMatch,
    outputs.validationState,
    outputs.digitalSourceType,
    outputs.issuer,
    outputs.commonName,
    outputs.softwareAgent,
    outputs.signingTime,
    outputs.certFingerprint,
  ]) {
    const strBuf = Buffer.from(s, "utf8");
    const lenBuf = Buffer.alloc(8);
    lenBuf.writeBigUInt64LE(BigInt(strBuf.length));
    parts.push(lenBuf);
    parts.push(strBuf);
  }

  return Buffer.concat(parts);
}

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
    // Empty proof: skip-verification feature active in test builds
    const proof = Buffer.alloc(0);
    const publicInputs = encodeBincodePublicOutputs({
      contentHash,
      hasC2pa: true,
      trustListMatch: "official",
      validationState: "Trusted",
      digitalSourceType:
        "http://cv.iptc.org/newscodes/digitalsourcetype/trainedAlgorithmicMedia",
      issuer: "OpenAI",
      commonName: "Truepic Lens CLI in Sora",
      softwareAgent: "GPT-4o",
      signingTime: "2024-12-01T00:00:00Z",
      certFingerprint: "abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234",
    });

    const computeBudgetIx = ComputeBudgetProgram.setComputeUnitLimit({
      units: 400_000,
    });

    const tx = await (program.methods as any)
      .submitProof(proof, publicInputs, Array.from(contentHash))
      .accounts({
        attestation: attestationPda,
        submitter: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .preInstructions([computeBudgetIx])
      .rpc();

    console.log("    tx:", tx);

    // Fetch and verify — Anchor uses camelCase field names
    const attestation: any = await (
      program.account as any
    ).attestation.fetch(attestationPda);

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
    expect(attestation.certFingerprint).to.equal("abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234");
    expect(attestation.submittedBy.toBase58()).to.equal(
      provider.wallet.publicKey.toBase58()
    );
    expect(attestation.timestamp.toNumber()).to.be.greaterThan(0);
  });

  it("rejects duplicate attestation for same content hash", async () => {
    const publicInputs = encodeBincodePublicOutputs({
      contentHash,
      hasC2pa: true,
      trustListMatch: "official",
      validationState: "Trusted",
      digitalSourceType: "",
      issuer: "OpenAI",
      commonName: "Test",
      softwareAgent: "Test",
      signingTime: "",
      certFingerprint: "",
    });

    try {
      await (program.methods as any)
        .submitProof(Buffer.alloc(0), publicInputs, Array.from(contentHash))
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

  it("rejects content_hash mismatch", async () => {
    const badHash = Buffer.alloc(32, 0xee);
    const [badPda] = PublicKey.findProgramAddressSync(
      [ATTESTATION_SEED, badHash],
      program.programId
    );

    // public_inputs encodes a DIFFERENT content_hash than what we pass as arg
    const mismatchContentHash = Buffer.alloc(32, 0xdd);
    const publicInputs = encodeBincodePublicOutputs({
      contentHash: mismatchContentHash,
      hasC2pa: false,
      trustListMatch: "",
      validationState: "None",
      digitalSourceType: "",
      issuer: "",
      commonName: "",
      softwareAgent: "",
      signingTime: "",
      certFingerprint: "",
    });

    try {
      await (program.methods as any)
        .submitProof(Buffer.alloc(0), publicInputs, Array.from(badHash))
        .accounts({
          attestation: badPda,
          submitter: provider.wallet.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
      expect.fail("should have thrown");
    } catch (e: any) {
      expect(e.toString()).to.include("ContentHashMismatch");
    }
  });

  it("rejects strings that exceed max length", async () => {
    const badHash = Buffer.alloc(32, 0xff);
    const [badPda] = PublicKey.findProgramAddressSync(
      [ATTESTATION_SEED, badHash],
      program.programId
    );

    const publicInputs = encodeBincodePublicOutputs({
      contentHash: badHash,
      hasC2pa: false,
      trustListMatch: "x".repeat(129), // exceeds MAX_STRING_LEN
      validationState: "Valid",
      digitalSourceType: "",
      issuer: "",
      commonName: "",
      softwareAgent: "",
      signingTime: "",
      certFingerprint: "",
    });

    try {
      await (program.methods as any)
        .submitProof(Buffer.alloc(0), publicInputs, Array.from(badHash))
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
