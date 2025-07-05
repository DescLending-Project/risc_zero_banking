use alloy::{
    network::EthereumWallet,
    providers::ProviderBuilder,
    signers::local::PrivateKeySigner,
};
use alloy_primitives::Address;
use anyhow::Result;
use clap::Parser;
use std::{env, fs::File, io::Read};
use bincode;
use serde::{Deserialize, Serialize};
use methods::GUEST_ELF;
use risc0_ethereum_contracts::encode_seal;
use risc0_zkvm::{
    default_prover, ExecutorEnv, Receipt,
    ProverOpts, VerifierContext,
};
use ethereum_types::{H256, U256};
use url::Url;

pub mod credit_score_abi {
    alloy::sol!(
        #[sol(rpc, all_derives)]
        interface ICreditScore {
            function submitCreditScore(
                uint64 score,
                string calldata serverName,
                bytes calldata seal
            ) external;
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
    contract: Address,

    #[clap(long)]
    first_receipt_path: String,

    #[clap(long)]
    second_receipt_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct VerificationOutput {
    is_valid: bool,
    server_name: String,
    score: Option<u64>,
    error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProofOutput {
    exists: bool,
    nonce: Option<U256>,
    balance: Option<U256>,
    storage_root: Option<H256>,
    code_hash: Option<H256>,
    storage_value: Option<U256>,
}

#[derive(Debug, Serialize, Deserialize)]
struct HybridCreditScore {
    score: u64,
    server_name: String,
}

fn main() -> Result<()> {
    env_logger::init();
    
    // Try CLI args first, fallback to env args for local testing
    let args = if env::args().len() > 1 {
        Args::parse()
    } else {
        // Fallback for local testing without CLI args
        let mut env_args = env::args();
        let exe = env_args.next().unwrap();
        let first_receipt_path = env_args.next().unwrap_or_else(|| {
            eprintln!("Usage: {} <first_receipt_path> <second_receipt_path> [--chain-id <id> --rpc-url <url> --contract <addr> --eth-wallet-private-key <key>]", exe);
            std::process::exit(1);
        });
        let second_receipt_path = env_args.next().unwrap_or_else(|| {
            eprintln!("Usage: {} <first_receipt_path> <second_receipt_path> [--chain-id <id> --rpc-url <url> --contract <addr> --eth-wallet-private-key <key>]", exe);
            std::process::exit(1);
        });

        // Just run local proof generation if no blockchain args provided
        return run_local_proof(&first_receipt_path, &second_receipt_path);
    };

    // Load and process receipts
    let (first_receipt, second_receipt, first_journal_bytes, second_journal_bytes) = 
        load_receipts(&args.first_receipt_path, &args.second_receipt_path)?;

    // Check proof validity for display
    let tradfi_valid = first_receipt.journal.decode::<VerificationOutput>()
        .map(|output| output.is_valid)
        .unwrap_or(false);

    let account_valid = second_receipt.journal.decode::<ProofOutput>()
        .map(|output| output.exists)
        .unwrap_or(false);

    println!("TRADFI_TLSN_PROOF {}", if tradfi_valid { "valid" } else { "invalid" });
    println!("ETH_ACCOUNT_PROOF {}", if account_valid { "valid" } else { "invalid" });

    // Build the zkVM execution environment
    let env = ExecutorEnv::builder()
        .add_assumption(first_receipt)
        .add_assumption(second_receipt)
        .write(&first_journal_bytes)?
        .write(&second_journal_bytes)?
        .build()?;

    // Generate Groth16 proof for on-chain verification
    let receipt = default_prover()
        .prove_with_ctx(
            env, 
            &VerifierContext::default(), 
            GUEST_ELF, 
            &ProverOpts::groth16()
        )?
        .receipt;

    // Decode the result
    let result: HybridCreditScore = receipt.journal.decode()?;
    
    println!("credit score={}", result.score);
    println!("fetched from server={}", result.server_name);

    // Encode the seal for on-chain verification
    let seal = encode_seal(&receipt)?;

    // Submit to blockchain
    let wallet = EthereumWallet::from(args.eth_wallet_private_key);
    let provider = ProviderBuilder::new().wallet(wallet).connect_http(args.rpc_url);

    let contract = ICreditScore::new(args.contract, provider);
    
    let call = contract.submitCreditScore(
        result.score,
        result.server_name,
        seal.into()
    );
    
    let runtime = tokio::runtime::Runtime::new()?;
    let pending_tx = runtime.block_on(call.send())?;
    let tx_receipt = runtime.block_on(pending_tx.get_receipt())?;
    
    println!("Credit score submitted on-chain: {:?}", tx_receipt.transaction_hash);

    Ok(())
}

fn run_local_proof(first_receipt_path: &str, second_receipt_path: &str) -> Result<()> {
    let (first_receipt, second_receipt, first_journal_bytes, second_journal_bytes) = 
        load_receipts(first_receipt_path, second_receipt_path)?;

    // Check proof validity
    let tradfi_valid = first_receipt.journal.decode::<VerificationOutput>()
        .map(|output| output.is_valid)
        .unwrap_or(false);

    let account_valid = second_receipt.journal.decode::<ProofOutput>()
        .map(|output| output.exists)
        .unwrap_or(false);

    println!("TRADFI_PROOF {}", if tradfi_valid { "valid" } else { "invalid" });
    println!("ACCOUNT_PROOF {}", if account_valid { "valid" } else { "invalid" });

    // Build execution environment
    let env = ExecutorEnv::builder()
        .add_assumption(first_receipt)
        .add_assumption(second_receipt)
        .write(&first_journal_bytes)?
        .write(&second_journal_bytes)?
        .build()?;

    // Generate local proof (fast proving for testing)
    let opts = risc0_zkvm::ProverOpts::default().with_receipt_kind(risc0_zkvm::ReceiptKind::Succinct);
    let prove_info = default_prover()
        .prove_with_opts(env, GUEST_ELF, &opts)?;

    let result: HybridCreditScore = prove_info.receipt.journal.decode()?;
    
    println!("credit score={}", result.score);
    println!("fetched from server={}", result.server_name);

    Ok(())
}

fn load_receipts(first_path: &str, second_path: &str) -> Result<(Receipt, Receipt, Vec<u8>, Vec<u8>)> {
    // Load first receipt
    let mut file = File::open(first_path)?;
    let mut first_receipt_bytes = Vec::new();
    file.read_to_end(&mut first_receipt_bytes)?;
    let first_receipt: Receipt = bincode::deserialize(&first_receipt_bytes)?;

    // Load second receipt  
    let mut file = File::open(second_path)?;
    let mut second_receipt_bytes = Vec::new();
    file.read_to_end(&mut second_receipt_bytes)?;
    let second_receipt: Receipt = bincode::deserialize(&second_receipt_bytes)?;

    // Extract journal bytes
    let first_journal_bytes = first_receipt.journal.bytes.clone();
    let second_journal_bytes = second_receipt.journal.bytes.clone();

    Ok((first_receipt, second_receipt, first_journal_bytes, second_journal_bytes))
}