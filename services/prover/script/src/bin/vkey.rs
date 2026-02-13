use sp1_sdk::{include_elf, HashableKey, Prover, ProverClient};

const ELF: &[u8] = include_elf!("provenance-program");

fn main() {
    let client = ProverClient::builder().mock().build();
    let (_, vk) = client.setup(ELF);
    println!("{}", vk.bytes32());
}
