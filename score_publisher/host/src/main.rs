use alloy::{
    network::EthereumWallet,
    providers::ProviderBuilder,
    signers::local::PrivateKeySigner,
};
use alloy_primitives::Address as AlloyAddress;
use alloy_sol_types::{SolValue, sol};
use anyhow::Result;
use clap::Parser;
use std::{fs::File, io::Read};
use serde::{Deserialize, Serialize};
use methods::GUEST_ELF;
use risc0_ethereum_contracts::encode_seal;
use risc0_zkvm::{default_prover, ExecutorEnv, ProverOpts, Receipt, ReceiptKind};
use url::Url;

sol! {
    struct JournalData {
        uint64 score;
        string serverName;
        string stateRootProvider; 
        uint64 blockNumber;
        string userIdHash;           
        uint64 tradfiDateTimestamp; 
        string userAddress;
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
                string userIdHash;           
                uint64 tradfiDateTimestamp;  
                string userAddress;
                bytes32[] allNullifiers;
            }
            
            function submitCreditScore(JournalData calldata journalData, bytes calldata seal) external;
            
            function getCreditScore(address user) external view returns (
                uint64 score,
                bool isValid,
                uint256 timestamp,
                string memory userAddress,
                bytes32[] memory allNullifiers
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
    tradfi_receipt_path: String,
    
    #[clap(long)]
    account_receipt_path: String,
    
    #[clap(long)]
    stateroot_receipt_path: String,
}


#[derive(Debug, Serialize, Deserialize)]
struct VerificationOutput {
    is_valid: bool,
    server_name: String,
    score: Option<u64>,
    user_id_hash: Option<String>, 
    tradfi_date_timestamp: Option<u64>, 
    error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct StateRootOutput {
    is_valid: bool,
    server_name: String,
    state_root: Option<String>,
    block_number: Option<String>,
    error: Option<String>,
}

fn load_receipt(receipt_path: &str) -> Result<(Receipt, Vec<u8>)> {
    let mut file = File::open(receipt_path)?;
    let mut receipt_bytes = Vec::new();
    file.read_to_end(&mut receipt_bytes)?;
    
    let receipt: Receipt = bincode::deserialize(&receipt_bytes)
        .map_err(|e| anyhow::anyhow!("Failed to deserialize receipt: {}", e))?;

    let journal_bytes = receipt.journal.bytes.clone();
    Ok((receipt, journal_bytes))
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

    // Load all inner receipts
    let (tradfi_receipt, tradfi_journal_bytes) = load_receipt(&args.tradfi_receipt_path)?;
    let (defi_receipt, defi_journal_bytes) = load_receipt(&args.account_receipt_path)?;
    let (stateroot_receipt, stateroot_journal_bytes) = load_receipt(&args.stateroot_receipt_path)?;

    let env = ExecutorEnv::builder()
        .add_assumption(tradfi_receipt)
        .add_assumption(defi_receipt)
        .add_assumption(stateroot_receipt)
        .write(&tradfi_journal_bytes)?
        .write(&defi_journal_bytes)?
        .write(&stateroot_journal_bytes)?
        .build()?;

    println!("Starting proof generation...");
    
    let opts = ProverOpts::default().with_receipt_kind(ReceiptKind::Groth16);
    let prove_info = default_prover().prove_with_opts(env, GUEST_ELF, &opts)?;
    let receipt = prove_info.receipt;
    let receipt_bytes = bincode::serialize(&receipt)?;
    std::fs::write("hybrid_credit_score_receipt.bin", receipt_bytes)?;

    // Decode final journal
    let journal_struct = JournalData::abi_decode(&receipt.journal.bytes)?;

    println!("\n=== HYBRID CREDIT SCORE RESULTS ===");
    println!(" Final Hybrid Score: {}", journal_struct.score);
    println!(" TradFi Server: '{}'", journal_struct.serverName);
    println!(" State Root Provider: '{}'", journal_struct.stateRootProvider);
    println!(" Block Number: {}", journal_struct.blockNumber);
    println!(" User ID Hash: '{}'", journal_struct.userIdHash);
    println!(" TradFi Date Timestamp: {}", format_timestamp(journal_struct.tradfiDateTimestamp));
    println!(" User Address: '{}'", journal_struct.userAddress);
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
        userIdHash: journal_struct.userIdHash,                 
        tradfiDateTimestamp: journal_struct.tradfiDateTimestamp, 
        userAddress: journal_struct.userAddress,
        allNullifiers: journal_struct.allNullifiers,
    };

    // Submit to contract
    let seal = encode_seal(&receipt)?;
    let wallet = EthereumWallet::from(args.eth_wallet_private_key);
    let provider = ProviderBuilder::new().wallet(wallet).connect_http(args.rpc_url);
    let contract = ICreditScore::new(args.contract, provider);

    println!("\n=== SUBMITTING TO BLOCKCHAIN ===");
    println!(" About to call submitCreditScore...");
    println!(" Contract Address: {:?}", args.contract);
    
    let runtime = tokio::runtime::Runtime::new()?;
    let call = contract.submitCreditScore(contract_journal_data, seal.into());
    
    let pending_tx = runtime.block_on(call.send())?;
    let tx_receipt = runtime.block_on(pending_tx.get_receipt())?;

    println!("✅ Successfully submitted to blockchain!");
    println!("   TX Hash: {:?}", tx_receipt.transaction_hash);
    println!("   Block Number: {:?}", tx_receipt.block_number);
    println!("   Gas Used: {:?}", tx_receipt.gas_used);

    Ok(())
}