use anyhow::{Context, Result};
use bincode;
use ethereum_types::{H256, U256};
use ethers::prelude::*;
use risc0_groth16::docker::stark_to_snark;
use risc0_zkvm::Prover;
use risc0_zkvm::{
    default_executor, default_prover, serde::from_slice, ExecutorEnv, ProverOpts, Receipt,
    ReceiptKind,
};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::env;
use std::fs;

// Import the generated method ID
use methods::ACCOUNT_MERKEL_PROOF_ID;
use methods::ACCOUNT_MERKEL_PROOF_PATH;

// Match the structures defined in the guest
#[derive(Serialize, Deserialize)]
struct ProofInput {
    state_root: [u8; 32],
    address: [u8; 20],
    slot_key: Option<[u8; 32]>,
    account_proof: Vec<Vec<u8>>,
    storage_proof: Option<Vec<Vec<u8>>>,
}

#[derive(Deserialize)]
struct ProofOutput {
    exists: bool,
    nonce: Option<U256>,
    balance: Option<U256>,
    storage_root: Option<H256>,
    code_hash: Option<H256>,
    storage_value: Option<U256>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Try multiple provider options
    let provider = setup_eth_provider().await?;

    // Address we want to verify
    let address = "0x8d0BB74e37ab644964AcA2f3Fbe12b9147f9d841".parse::<Address>()?;

    // Optional storage slot
    let slot = Some(H256::zero()); // Example: first storage slot

    // Get a state root hash at a specific block
    let block_number = 22406754;
    let block = BlockId::Number(BlockNumber::Number(block_number.into()));

    println!("Getting block details for block: {}", block_number);
    let block_data = provider
        .get_block(block)
        .await?
        .context("Block not found")?;

    let state_root = block_data.state_root;
    println!("Got state root: 0x{}", hex::encode(state_root.as_bytes()));

    println!(
        "Getting proof for address: {} at block: {}",
        address, block_number
    );

    // Fetch the proof from the Ethereum node using eth_getProof
    let mut proof = provider
        .get_proof(address, vec![slot.unwrap_or_default()], Some(block))
        .await?;

    println!("Got proof, account has balance: {}", proof.balance);

    // Convert proof to the format expected by our verifier
    let account_proof: Vec<Vec<u8>> = proof
        .account_proof
        .into_iter()
        .map(|bytes| bytes.to_vec())
        .collect();

    // Only include storage proof if we specified a slot
    let storage_proof = if slot.is_some() {
        Some(
            proof.storage_proof[0]
                .proof
                .clone()
                .into_iter()
                .map(|bytes| bytes.to_vec())
                .collect(),
        )
    } else {
        None
    };

    // Prepare input for RISC Zero guest
    let input = ProofInput {
        state_root: state_root.into(),
        address: address.into(),
        slot_key: slot.map(|h| h.into()),
        account_proof,
        storage_proof,
    };

    // Read the ELF file
    println!("Reading ELF from: {}", ACCOUNT_MERKEL_PROOF_PATH);
    let elf_bytes = fs::read(ACCOUNT_MERKEL_PROOF_PATH)?;

    // For development: first run in the executor for faster debugging
    println!("Running executor for verification...");
    let exec_env = ExecutorEnv::builder().write(&input)?.build()?;

    // Use the execute method with the ELF bytes
    let exec = default_executor();
    let session = exec.execute(exec_env, &elf_bytes)?;

    // Decode the journal bytes
    let output: ProofOutput = from_slice(&session.journal.bytes)?;

    // Display the results from execution
    println!("Execution verification successful!");
    println!("Account exists: {}", output.exists);

    if output.exists {
        println!("Account details from execution:");
        println!("  Nonce: {}", output.nonce.unwrap());
        println!("  Balance: {}", output.balance.unwrap());
        println!(
            "  Storage Root: 0x{}",
            hex::encode(output.storage_root.unwrap().as_bytes())
        );
        println!(
            "  Code Hash: 0x{}",
            hex::encode(output.code_hash.unwrap().as_bytes())
        );

        if let Some(value) = output.storage_value {
            println!("Storage value: {}", value);
        }
    }

    // Now generate an actual ZK proof (slower but cryptographically secure)
    println!("\nGenerating ZK proof...");
    let prove_env = ExecutorEnv::builder().write(&input)?.build()?;

    let prover = default_prover();
    // Pass the ELF bytes
    let opts = ProverOpts::default().with_receipt_kind(ReceiptKind::Succinct);
    let receipt_info = prover.prove_with_opts(prove_env, &elf_bytes, &opts)?;

    // Access the receipt field within ProveInfo
    let receipt = &receipt_info.receipt;

    // storing receipt as bincode and json
    let serialized_receipt = bincode::serialize(&receipt)?;
    std::fs::write("complete_receipt.bin", serialized_receipt)?;
    let receipt_json = serde_json::to_string_pretty(&receipt)?;
    std::fs::write("receipt.json", receipt_json)?;

    // Verify the proof - use the receipt field
    println!("Verifying ZK proof...");
    receipt.verify(ACCOUNT_MERKEL_PROOF_ID)?;

    // Decode the journal from the receipt field
    let output: ProofOutput = from_slice(&receipt.journal.bytes)?;

    // Print the results
    println!("ZK proof verification successful!");
    println!("Account exists: {}", output.exists);

    if output.exists {
        println!("Account details from ZK proof:");
        println!("  Nonce: {}", output.nonce.unwrap());
        println!("  Balance: {}", output.balance.unwrap());
        println!(
            "  Storage Root: 0x{}",
            hex::encode(output.storage_root.unwrap().as_bytes())
        );
        println!(
            "  Code Hash: 0x{}",
            hex::encode(output.code_hash.unwrap().as_bytes())
        );

        if let Some(value) = output.storage_value {
            println!("Storage value: {}", value);
        }
    }

    Ok(())
}

// Function to try multiple provider options
async fn setup_eth_provider() -> Result<Provider<Http>> {
    // Try Alchemy if environment variable exists
    if let Ok(alchemy_key) = env::var("ALCHEMY_API_KEY") {
        let url = format!("https://eth-mainnet.alchemyapi.io/v2/{}", alchemy_key);
        let provider = Provider::<Http>::try_from(url.as_str())?;

        // Test the connection
        if let Ok(_) = provider.get_block_number().await {
            println!("Connected to Ethereum via Alchemy");
            return Ok(provider);
        }
    }
    // If all else fails, try Ethereum archive RPCs from eth-archive.r2
    let url = "https://eth-archive.r2.scorched.io/";
    println!("Trying fallback archive node: {}", url);
    let provider = Provider::<Http>::try_from(url)?;

    // Test the connection
    provider.get_block_number().await?;
    println!("Connected to Ethereum via archive node");

    Ok(provider)
}
