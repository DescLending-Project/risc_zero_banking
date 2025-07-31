use std::{env, fs, path::PathBuf};
use anyhow::Result;
use methods::{PROOF_VERIFIER_GUEST_ELF, PROOF_VERIFIER_GUEST_ID};
use risc0_zkvm::{default_prover, ExecutorEnv, ProverOpts, ReceiptKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct VerificationOutput {
    is_valid: bool,
    server_name: String,
    state_root: Option<String>,
    block_number: Option<String>,
    error: Option<String>,
}

fn main() -> Result<()> {
    let proof_path = env::args()
        .nth(1)
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("Usage: program <path-to-tlsn-proof.json>"))?;
    let proof_json = fs::read_to_string(&proof_path)?;
    
    let env = ExecutorEnv::builder().write(&proof_json)?.build()?;
    let prover = default_prover();
    
    println!("Running the prover...");
    let opts = ProverOpts::succinct();
    let prove_info = prover.prove_with_opts(env, PROOF_VERIFIER_GUEST_ELF, &opts)?;
    println!("Proving finished.");
    
    let receipt = prove_info.receipt;
    receipt.verify(PROOF_VERIFIER_GUEST_ID)?;
    println!("Receipt verification successful!");
    
    let output: VerificationOutput = receipt.journal.decode()?;
    println!("\nGuest Output:");
    println!(" Is Valid: {}", output.is_valid);
    println!(" Server Name: {}", output.server_name);
    
    if let Some(state_root) = &output.state_root {
        println!(" State Root: {}", state_root);
    }
    
    if let Some(block_number) = &output.block_number {
        println!(" Block Number: {}", block_number);
        
        if output.is_valid {
            println!("\nSuccessfully verified Alchemy response and extracted:");
            println!(" - State Root: {}", output.state_root.as_ref().unwrap_or(&"None".to_string()));
            println!(" - Block Number: {}", block_number);
        }
    }
    
    if let Some(err_msg) = output.error {
        println!(" Error: {}", err_msg);
    }
    
    println!("\nSaving receipt to alchemy_stateroot.bin...");
    let receipt_bytes = bincode::serialize(&receipt)?;
    fs::write("alchemy_stateroot.bin", &receipt_bytes)?;
    println!("Receipt saved to alchemy_stateroot.bin ({} bytes)", receipt_bytes.len());
    
    Ok(())
}