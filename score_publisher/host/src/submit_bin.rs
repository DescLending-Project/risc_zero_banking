use alloy::{
    network::EthereumWallet,
    providers::ProviderBuilder,
    signers::local::PrivateKeySigner,
};
use alloy_primitives::{Address as AlloyAddress};
use alloy_sol_types::{SolValue, sol};
use anyhow::Result;
use clap::Parser;
use std::{fs::File, io::Read};
use risc0_ethereum_contracts::encode_seal;
use risc0_zkvm::Receipt;
use url::Url;

sol! {
    struct JournalData {
        uint64 score;
        string serverName;
        string stateRootProvider; 
        uint64 blockNumber;
        bytes32 tradfiNullifier;           
        uint64 tradfiDateTimestamp; 
        address userAddress;
        bytes32[] allNullifiers;
    }
}

pub mod credit_score_abi {
    alloy::sol!(
        #[sol(rpc, all_derives)]
        interface ICreditScore {
            struct JournalData {
                uint64 score;
                string serverName;
                string stateRootProvider;
                uint64 blockNumber;
                bytes32 tradfiNullifier;           
                uint64 tradfiDateTimestamp;  
                address userAddress;
                bytes32[] allNullifiers;
            }
            
            function submitR0CreditScore(JournalData calldata journalData, bytes calldata seal) external;
            
            function getCreditScore(address user) external view returns (
                uint64 score,
                bool isUnused,
                uint256 timestamp
            );
        }
    );
}

use credit_score_abi::ICreditScore;

#[derive(Parser, Debug)]
struct Args {
    #[clap(long)]
    chain_id: u64,

    #[clap(long, env)]
    eth_wallet_private_key: PrivateKeySigner,

    #[clap(long)]
    rpc_url: Url,

    #[clap(long)]
    contract: AlloyAddress,

    #[clap(long)]
    proof_path: String,
}

fn load_existing_receipt(receipt_path: &str) -> Result<Receipt> {
    println!("Loading existing proof from: {}", receipt_path);
    
    let mut file = File::open(receipt_path)
        .map_err(|e| anyhow::anyhow!("Failed to open proof file '{}': {}", receipt_path, e))?;
    
    let mut receipt_bytes = Vec::new();
    file.read_to_end(&mut receipt_bytes)
        .map_err(|e| anyhow::anyhow!("Failed to read proof file: {}", e))?;
    
    let receipt: Receipt = bincode::deserialize(&receipt_bytes)
        .map_err(|e| anyhow::anyhow!("Failed to deserialize receipt: {}", e))?;

    println!("✅ Successfully loaded existing proof!");
    Ok(receipt)
}

// Helper function to format timestamp for display
fn format_timestamp(timestamp: u64) -> String {
    use std::time::{UNIX_EPOCH, Duration};
    
    match UNIX_EPOCH.checked_add(Duration::from_secs(timestamp)) {
        Some(_datetime) => {
            format!("Unix timestamp: {} (Date: {})", timestamp, timestamp)
        },
        None => "Invalid timestamp".to_string(),
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Load the existing proof
    let receipt = load_existing_receipt(&args.proof_path)?;

    // Decode the journal to see what we're submitting
    let journal_struct = JournalData::abi_decode(&receipt.journal.bytes)
        .map_err(|e| anyhow::anyhow!("Failed to decode journal: {}", e))?;

    println!("\n=== PROOF CONTENTS ===");
    println!(" Final Hybrid Score: {}", journal_struct.score);
    println!(" TradFi Server: '{}'", journal_struct.serverName);
    println!(" State Root Provider: '{}'", journal_struct.stateRootProvider);
    println!(" Block Number: {}", journal_struct.blockNumber);
    println!(" TradFi Nullifier: 0x{}", hex::encode(&journal_struct.tradfiNullifier));
    println!(" TradFi Date Timestamp: {}", format_timestamp(journal_struct.tradfiDateTimestamp));
    println!(" User Address: {:?}", journal_struct.userAddress);
    println!(" Nullifiers Count: {}", journal_struct.allNullifiers.len());
    if !journal_struct.allNullifiers.is_empty() {
        println!(" First Nullifier: 0x{}", hex::encode(&journal_struct.allNullifiers[0]));
    }

    // Convert to contract's JournalData type
    let contract_journal_data = ICreditScore::JournalData {
        score: journal_struct.score,
        serverName: journal_struct.serverName,
        stateRootProvider: journal_struct.stateRootProvider,
        blockNumber: journal_struct.blockNumber,
        tradfiNullifier: journal_struct.tradfiNullifier,                 
        tradfiDateTimestamp: journal_struct.tradfiDateTimestamp, 
        userAddress: journal_struct.userAddress,
        allNullifiers: journal_struct.allNullifiers,
    };

    // Encode the seal for submission
    let seal = encode_seal(&receipt)
        .map_err(|e| anyhow::anyhow!("Failed to encode seal: {}", e))?;

    // Setup blockchain connection
    let wallet = EthereumWallet::from(args.eth_wallet_private_key);
    let provider = ProviderBuilder::new().wallet(wallet).connect_http(args.rpc_url);
    let contract = ICreditScore::new(args.contract, provider);

    println!("\n=== SUBMITTING TO BLOCKCHAIN ===");
    println!(" About to call submitR0CreditScore...");
    println!(" Contract Address: {:?}", args.contract);
    println!(" Chain ID: {}", args.chain_id);
    
    let runtime = tokio::runtime::Runtime::new()?;
    let call = contract.submitR0CreditScore(contract_journal_data, seal.into());
    
    println!(" Sending transaction...");
    let pending_tx = runtime.block_on(call.send())
        .map_err(|e| anyhow::anyhow!("Failed to send transaction: {}", e))?;
    
    println!(" Waiting for confirmation...");
    let tx_receipt = runtime.block_on(pending_tx.get_receipt())
        .map_err(|e| anyhow::anyhow!("Failed to get transaction receipt: {}", e))?;

    println!("✅ Successfully submitted to blockchain!");
    println!("   TX Hash: {:?}", tx_receipt.transaction_hash);
    println!("   Block Number: {:?}", tx_receipt.block_number);
    println!("   Gas Used: {:?}", tx_receipt.gas_used);
    println!("   Status: {:?}", if tx_receipt.status() { "Success" } else { "Failed" });

    Ok(())
}