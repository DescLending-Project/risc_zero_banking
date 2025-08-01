use alloy::{
    network::EthereumWallet,
    providers::ProviderBuilder,
    signers::local::PrivateKeySigner,
};
use alloy_primitives::Address;
use anyhow::Result;
use clap::Parser;
use std::{fs::File, io::Read};
use serde::{Deserialize, Serialize};
use methods::GUEST_ELF;
use risc0_ethereum_contracts::encode_seal;
use risc0_zkvm::{default_prover, ExecutorEnv, ProverOpts, Receipt, ReceiptKind};
use url::Url;
use ethereum_types::{U256, H256};

pub mod credit_score_abi {
    alloy::sol!(
        #[sol(rpc, all_derives)]
        interface ICreditScore {
            function submitCreditScore(
                uint64 score,
                string calldata serverName,
                string calldata stateRootProvider,
                bytes calldata seal,
                bytes calldata journalData
            ) external;
            
            function testVerify(
                bytes calldata seal,
                bytes calldata journalData
            ) external view returns (bool);
            
            function isServerAuthorized(string calldata serverName) external view returns (bool);
            
            function isStateRootProviderAuthorized(string calldata providerName) external view returns (bool);
            
            function imageId() external view returns (bytes32);
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
struct StateRootOutput {
    is_valid: bool,
    server_name: String,
    state_root: Option<String>,
    block_number: Option<String>,
    error: Option<String>,
}

fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();

    let (tradfi_receipt, tradfi_journal_bytes) = load_receipt(&args.tradfi_receipt_path)?;
    let (account_receipt, account_journal_bytes) = load_receipt(&args.account_receipt_path)?;
    let (stateroot_receipt, stateroot_journal_bytes) = load_receipt(&args.stateroot_receipt_path)?;

    println!("🔍 Attempting to decode TradFi receipt journal...");
    let tradfi_valid = match tradfi_receipt.journal.decode::<VerificationOutput>() {
        Ok(output) => {
            println!("✅ TradFi receipt decoded successfully");
            println!("  - is_valid: {}", output.is_valid);
            println!("  - server_name: '{}'", output.server_name);
            println!("  - score: {:?}", output.score);
            output.is_valid
        }
        Err(e) => return Err(anyhow::anyhow!("Failed to decode TradFi receipt journal: {}", e)),
    };

    println!("🔍 Attempting to decode Account receipt journal...");
    let account_valid = match account_receipt.journal.decode::<ProofOutput>() {
        Ok(output) => {
            println!("✅ Account receipt decoded successfully");
            println!("  - exists: {}", output.exists);
            println!("  - balance: {:?}", output.balance);
            println!("  - nonce: {:?}", output.nonce);
            output.exists
        }
        Err(e) => return Err(anyhow::anyhow!("Failed to decode Account receipt journal: {}", e)),
    };

    println!("🔍 Attempting to decode StateRoot receipt journal...");
    let stateroot_valid = match stateroot_receipt.journal.decode::<StateRootOutput>() {
        Ok(output) => {
            println!("✅ StateRoot receipt decoded successfully");
            println!("  - is_valid: {}", output.is_valid);
            println!("  - server_name: '{}'", output.server_name);
            println!("  - state_root: {:?}", output.state_root);
            println!("  - block_number: {:?}", output.block_number);
            output.is_valid
        }
        Err(e) => return Err(anyhow::anyhow!("Failed to decode StateRoot receipt journal: {}", e)),
    };

    println!("TRADFI_PROOF {}", if tradfi_valid { "valid" } else { "invalid" });
    println!("ACCOUNT_PROOF {}", if account_valid { "valid" } else { "invalid" });
    println!("STATEROOT_PROOF {}", if stateroot_valid { "valid" } else { "invalid" });

    println!("🔍 Building ExecutorEnv with all three assumptions...");
    let env = ExecutorEnv::builder()
        .add_assumption(tradfi_receipt)
        .add_assumption(account_receipt)
        .add_assumption(stateroot_receipt)
        .write(&tradfi_journal_bytes)?
        .write(&account_journal_bytes)?
        .write(&stateroot_journal_bytes)?
        .build()?;

    println!("🔍 Starting proof generation...");
    let opts = ProverOpts::default().with_receipt_kind(ReceiptKind::Groth16);
    
    let prove_info = match default_prover().prove_with_opts(env, GUEST_ELF, &opts) {
        Ok(info) => {
            println!("✅ Proof generation successful");
            info
        }
        Err(e) => return Err(anyhow::anyhow!("Proof generation failed: {}", e)),
    };

    let receipt = prove_info.receipt;
    
    println!("🔍 Analyzing receipt type...");
    match &receipt.inner {
        risc0_zkvm::InnerReceipt::Groth16(_) => {
            println!("✅ Groth16 receipt generated successfully");
        }
        _ => {
            println!("❌ Expected Groth16 receipt but got different type");
        }
    }

    println!("🔍 Attempting to decode final receipt journal...");
    let committed_data_vec: Vec<u8> = receipt.journal.decode()?;

    if committed_data_vec.len() != 128 {
        return Err(anyhow::anyhow!("Expected 128 bytes in journal, got {}", committed_data_vec.len()));
    }
    let mut committed_data = [0u8; 128];
    committed_data.copy_from_slice(&committed_data_vec);

    let score = u64::from_le_bytes(committed_data[0..8].try_into()?);
    let server_name_bytes = &committed_data[8..56];
    let server_end_pos = server_name_bytes.iter().position(|&b| b == 0).unwrap_or(server_name_bytes.len());
    let server_name = String::from_utf8_lossy(&server_name_bytes[..server_end_pos]).to_string();

    let provider_name_bytes = &committed_data[56..104];
    let provider_end_pos = provider_name_bytes.iter().position(|&b| b == 0).unwrap_or(provider_name_bytes.len());
    let state_root_provider = String::from_utf8_lossy(&provider_name_bytes[..provider_end_pos]).to_string();

    let block_number = u64::from_le_bytes(committed_data[104..112].try_into()?);

    println!("✅ Final Hybrid Score: {}", score);
    println!("✅ TradFi Server: '{}'", server_name);
    println!("✅ State Root Provider: '{}'", state_root_provider);
    println!("✅ Block Number: {}", block_number);

    let final_server_name = if server_name.is_empty() { "unknown".to_string() } else { server_name };
    let final_state_root_provider = if state_root_provider.is_empty() { "unknown".to_string() } else { state_root_provider };

    let journal_bytes = &receipt.journal.bytes;
    let seal = encode_seal(&receipt)?;

    let wallet = EthereumWallet::from(args.eth_wallet_private_key);
    let provider = ProviderBuilder::new().wallet(wallet).connect_http(args.rpc_url);
    let contract = ICreditScore::new(args.contract, provider);

    let runtime = tokio::runtime::Runtime::new()?;

    println!("✅ About to call submitCreditScore...");
    let call = contract.submitCreditScore(
        score,
        final_server_name.clone(),
        final_state_root_provider.clone(),
        seal.into(),
        journal_bytes.clone().into(),
    );

    let pending_tx = runtime.block_on(call.send())?;
    let tx_receipt = runtime.block_on(pending_tx.get_receipt())?;

    println!("✅ On-chain TX hash: {:?}", tx_receipt.transaction_hash);

    Ok(())
}

fn load_receipt(receipt_path: &str) -> Result<(Receipt, Vec<u8>)> {
    println!("🔍 Loading receipt from: {}", receipt_path);
    let mut file = File::open(receipt_path)?;
    let mut receipt_bytes = Vec::new();
    file.read_to_end(&mut receipt_bytes)?;
    
    let receipt: Receipt = bincode::deserialize(&receipt_bytes)
        .map_err(|e| anyhow::anyhow!("Failed to deserialize receipt: {}", e))?;

    let journal_bytes = receipt.journal.bytes.clone();
    Ok((receipt, journal_bytes))
}