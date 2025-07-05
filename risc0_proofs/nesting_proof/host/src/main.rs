use std::{env, fs::File, io::Read};
use tracing_subscriber;
use bincode;
use serde::{Deserialize, Serialize};
use nesting_methods::NESTING_GUEST_ELF;
use risc0_zkvm::{
    default_prover, ExecutorEnv, Receipt,
    ProverOpts, ReceiptKind,
};
use ethereum_types::{H256, U256};

#[derive(Debug, Serialize, Deserialize)]
struct VerificationOutput {
    is_valid: bool,
    server_name: String,
    score: Option<u64>,
    error: Option<String>,
}

// Output structure from the Ethereum account merkle proof
#[derive(Debug, Serialize, Deserialize)]
struct ProofOutput {
    exists: bool,
    nonce: Option<U256>,
    balance: Option<U256>,
    storage_root: Option<H256>,
    code_hash: Option<H256>,
    storage_value: Option<U256>,
}

fn main() {
    // — Init logging —
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
        .init();

    // — CLI args —
    let mut args = env::args();
    let exe = args.next().unwrap();
    let first_receipt_path = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("Usage: {} <first_receipt_path> <second_receipt_path>", exe);
            std::process::exit(1);
        }
    };
    let second_receipt_path = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("Usage: {} <first_receipt_path> <second_receipt_path>", exe);
            std::process::exit(1);
        }
    };

    // — Load & deserialize the first receipt —
    let mut file = File::open(&first_receipt_path).expect("Failed to open first receipt");
    let mut first_receipt_bytes = Vec::new();
    file.read_to_end(&mut first_receipt_bytes).unwrap();
    let first_receipt: Receipt = bincode::deserialize(&first_receipt_bytes)
        .expect("Failed to deserialize first receipt");

    // — Load & deserialize the second receipt —
    let mut file = File::open(&second_receipt_path).expect("Failed to open second receipt");
    let mut second_receipt_bytes = Vec::new();
    file.read_to_end(&mut second_receipt_bytes).unwrap();
    let second_receipt: Receipt = bincode::deserialize(&second_receipt_bytes)
        .expect("Failed to deserialize second receipt");

    // — Check first receipt validity —
    let tradfi_valid = match first_receipt.journal.decode::<VerificationOutput>() {
        Ok(output) => output.is_valid,
        Err(_) => false,
    };

    // — Check second receipt validity —
    let account_valid = match second_receipt.journal.decode::<ProofOutput>() {
        Ok(output) => output.exists,
        Err(_) => false,
    };

    println!("TRADFI_PROOF {}", if tradfi_valid { "valid" } else { "invalid" });
    println!("ACCOUNT_PROOF {}", if account_valid { "valid" } else { "invalid" });

    // Extract journal bytes before moving the receipts
    let first_journal_bytes = first_receipt.journal.bytes.clone();
    let second_journal_bytes = second_receipt.journal.bytes.clone();

    // Build the zkVM execution environment with both receipts as assumptions
    let env = ExecutorEnv::builder()
        .add_assumption(first_receipt)
        .add_assumption(second_receipt)
        .write(&first_journal_bytes).unwrap()
        .write(&second_journal_bytes).unwrap()
        .build().unwrap();

    let opts = ProverOpts::default().with_receipt_kind(ReceiptKind::Succinct);
    
    let prove_info = default_prover()
        .prove_with_opts(env, NESTING_GUEST_ELF, &opts)
        .unwrap_or_else(|e| panic!("🔴 Nested proof failed: {:?}", e));

    let nested_receipt = prove_info.receipt;
    
    // Decode the minimal hybrid credit score result
    #[derive(Debug, Serialize, Deserialize)]
    struct HybridCreditScore {
        score: u64,           // Final hybrid score (0-850 like FICO)
        server_name: String,  // For on-chain server verification
    }
    
    let result: HybridCreditScore = nested_receipt.journal.decode().unwrap();
    
    println!("credit score={}", result.score);
    println!("fetched from server={}", result.server_name);
}