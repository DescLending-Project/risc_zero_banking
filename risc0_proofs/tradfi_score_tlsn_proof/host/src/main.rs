use anyhow::Result;
use methods::{PROOF_VERIFIER_GUEST_ELF, PROOF_VERIFIER_GUEST_ID};
use risc0_zkvm::{default_prover, ExecutorEnv, ProverOpts, ReceiptKind};
use serde::{Deserialize, Serialize};
use std::{env, fs, path::PathBuf, time::Instant};

#[derive(Debug, Serialize, Deserialize)]
struct VerificationOutput {
    is_valid: bool,
    server_name: String,
    score: Option<u64>,
    user_id_hash: Option<String>,
    tradfi_date_timestamp: Option<u64>,
    error: Option<String>,
}

fn main() -> Result<()> {
    let proof_path = env::args()
        .nth(1)
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("Usage: program <path-to-proof.json>"))?;

    let proof_json = fs::read_to_string(&proof_path)?;
    let env = ExecutorEnv::builder().write(&proof_json)?.build()?;

    let prover = default_prover();
    println!("Running the prover...");
    let start = Instant::now();

    let opts = ProverOpts::succinct();
    let prove_info = prover.prove_with_opts(env, PROOF_VERIFIER_GUEST_ELF, &opts)?;
    println!("Proving finished.");
    let duration = start.elapsed();
    println!("{} :Proving time in ms", duration.as_millis());

    let receipt = prove_info.receipt;
    receipt.verify(PROOF_VERIFIER_GUEST_ID)?;
    println!("Receipt verification successful!");

    let output: VerificationOutput = receipt.journal.decode()?;

    println!("\nGuest Output:");
    println!(" Is Valid: {}", output.is_valid);
    println!(" Server Name: {}", output.server_name);

    if let Some(score) = output.score {
        println!(" Score: {}", score);
        if output.is_valid && score > 5 {
            println!(
                "\n✅ Successfully verified proof and extracted score: {}",
                score
            );
        }
    }

    if let Some(user_id_hash) = &output.user_id_hash {
        println!(" User ID Hash: {}", user_id_hash);
    }

    if let Some(timestamp) = output.tradfi_date_timestamp {
        println!(
            " TradFi Date Timestamp: {} ({})",
            timestamp,
            format_date_timestamp(timestamp)
        );
    }

    if let Some(err_msg) = &output.error {
        println!(" Error: {}", err_msg);
    }

    // Show summary
    if output.is_valid {
        println!("\nTradFi Data Summary:");
        println!(" Verification: PASSED");
        println!(" Server: {}", output.server_name);

        if let Some(score) = output.score {
            println!(" Credit Score: {}", score);
        }

        if let Some(user_id_hash) = &output.user_id_hash {
            println!(" User ID Hash: {}", user_id_hash);
        }

        if let Some(timestamp) = output.tradfi_date_timestamp {
            println!(
                " TradFi Date Timestamp: {} ({})",
                timestamp,
                format_date_timestamp(timestamp)
            );
        }
    } else {
        println!("\n❌ TradFi verification FAILED");
        if let Some(err_msg) = &output.error {
            println!(" Error: {}", err_msg);
        }
    }

    println!("\nSaving receipt to tradfi_score.bin...");
    let receipt_bytes = bincode::serialize(&receipt)?;
    fs::write("tradfi_score.bin", &receipt_bytes)?;
    println!(
        "Receipt saved to tradfi_score.bin ({} bytes)",
        receipt_bytes.len()
    );

    Ok(())
}

// Helper function to format date timestamp for display
fn format_date_timestamp(timestamp: u64) -> String {
    use std::time::{Duration, UNIX_EPOCH};

    match UNIX_EPOCH.checked_add(Duration::from_secs(timestamp)) {
        Some(datetime) => {
            format!("Date timestamp: {} (00:00:00 UTC)", timestamp)
        }
        None => "Invalid timestamp".to_string(),
    }
}

