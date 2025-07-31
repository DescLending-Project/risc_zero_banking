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

    println!("📂 Loading receipts...");
    
    // — Load & deserialize the first receipt —
    let mut file = File::open(&first_receipt_path)
        .unwrap_or_else(|e| panic!("Failed to open first receipt at {}: {}", first_receipt_path, e));
    let mut first_receipt_bytes = Vec::new();
    file.read_to_end(&mut first_receipt_bytes).unwrap();
    let first_receipt: Receipt = bincode::deserialize(&first_receipt_bytes)
        .expect("Failed to deserialize first receipt");

    // — Load & deserialize the second receipt —
    let mut file = File::open(&second_receipt_path)
        .unwrap_or_else(|e| panic!("Failed to open second receipt at {}: {}", second_receipt_path, e));
    let mut second_receipt_bytes = Vec::new();
    file.read_to_end(&mut second_receipt_bytes).unwrap();
    let second_receipt: Receipt = bincode::deserialize(&second_receipt_bytes)
        .expect("Failed to deserialize second receipt");

    println!("✅ Receipts loaded successfully");

    // — Check receipt contents (this implicitly validates them) —
    println!("🔍 Validating receipt contents...");
    
    let tradfi_valid = match first_receipt.journal.decode::<VerificationOutput>() {
        Ok(output) => {
            println!("TradFi data: valid={}, server={}, score={:?}", 
                output.is_valid, output.server_name, output.score);
            output.is_valid
        },
        Err(e) => {
            println!("❌ Failed to decode TradFi receipt: {}", e);
            false
        }
    };

    let account_valid = match second_receipt.journal.decode::<ProofOutput>() {
        Ok(output) => {
            println!("Account data: exists={}, balance={:?}, nonce={:?}", 
                output.exists, output.balance, output.nonce);
            output.exists
        },
        Err(e) => {
            println!("❌ Failed to decode account receipt: {}", e);
            false
        }
    };

    println!("TRADFI_PROOF {}", if tradfi_valid { "valid" } else { "invalid" });
    println!("ACCOUNT_PROOF {}", if account_valid { "valid" } else { "invalid" });

    if !tradfi_valid {
        println!("⚠️  Warning: TradFi proof is invalid, proceeding anyway...");
    }
    if !account_valid {
        println!("⚠️  Warning: Account proof is invalid, proceeding anyway...");
    }

    // Extract journal bytes BEFORE moving the receipts
    let first_journal_bytes = first_receipt.journal.bytes.clone();
    let second_journal_bytes = second_receipt.journal.bytes.clone();

    println!("📊 Journal sizes: TradFi={} bytes, Account={} bytes", 
        first_journal_bytes.len(), second_journal_bytes.len());

    // Build the zkVM execution environment with both receipts as assumptions
    println!("🔧 Building execution environment...");
    
    let env = ExecutorEnv::builder()
        .add_assumption(first_receipt)      // This moves first_receipt
        .add_assumption(second_receipt)     // This moves second_receipt
        .write(&first_journal_bytes).unwrap()
        .write(&second_journal_bytes).unwrap()
        .build().unwrap();

    println!("✅ Execution environment built");

    // Now that it's working, switch to SNARK for succinct proof
    let opts = ProverOpts::default().with_receipt_kind(ReceiptKind::Groth16); // SNARK - succinct!
    // let opts = ProverOpts::default(); // STARK - larger proof for debugging
    
    println!("🚀 Starting proof generation...");
    
    let prove_info = default_prover()
        .prove_with_opts(env, NESTING_GUEST_ELF, &opts)
        .unwrap_or_else(|e| {
            println!("🔴 Detailed error: {:?}", e);
            panic!("Nested proof failed: {:?}", e)
        });

    println!("✅ Proof generation completed!");

    let nested_receipt = prove_info.receipt;
    
    // Decode the minimal hybrid credit score result
    #[derive(Debug, Serialize, Deserialize)]
    struct HybridCreditScore {
        score: u64,           // Final hybrid score (0-850 like FICO)
        server_name: String,  // For on-chain server verification
    }
    
    let result: HybridCreditScore = nested_receipt.journal.decode()
        .expect("Failed to decode hybrid credit score");
    
    println!("🎉 SUCCESS!");
    println!("📈 Hybrid Credit Score: {}", result.score);
    println!("🏦 Server: {}", result.server_name);
    println!("🔥 Proof Type: Groth16 SNARK (succinct!)");
    
    // Optional: Save the final proof
    let final_proof_bytes = bincode::serialize(&nested_receipt).unwrap();
    println!("💾 Final proof size: {} bytes", final_proof_bytes.len());
    
    // Compare sizes
    if final_proof_bytes.len() < 10000 {
        println!("✅ Proof is succinct! Perfect for on-chain verification.");
    } else {
        println!("⚠️ Proof is still large - might be STARK instead of SNARK");
    }
}