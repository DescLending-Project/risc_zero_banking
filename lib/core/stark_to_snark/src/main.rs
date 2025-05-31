use anyhow::Result;
use risc0_groth16::docker::stark_to_snark;
use risc0_zkvm::Receipt;
use serde_json;
use std::fs;
use std::time::Instant;

/// Convert STARK receipt to SNARK pr/// Load Receipt from binary file (bincode format)
pub fn load_receipt_from_binary(file_path: &str) -> Result<Receipt> {
    // Check if file exists
    if !std::path::Path::new(file_path).exists() {
        return Err(anyhow::anyhow!("File not found: {}", file_path));
    }

    // Read file with detailed error context
    let data = fs::read(file_path)
        .map_err(|e| anyhow::anyhow!("Failed to read binary file '{}': {}", file_path, e))?;

    // Check if file is empty
    if data.is_empty() {
        return Err(anyhow::anyhow!("File is empty: {}", file_path));
    }

    // Deserialize with error context
    let receipt: Receipt = bincode::deserialize(&data)
        .map_err(|e| anyhow::anyhow!("Failed to deserialize receipt from '{}': {}. File may be corrupted or not a valid receipt.", file_path, e))?;

    Ok(receipt)
}

/// Load Receipt from JSON file
pub fn load_receipt_from_json(file_path: &str) -> Result<Receipt> {
    // Check if file exists
    if !std::path::Path::new(file_path).exists() {
        return Err(anyhow::anyhow!("File not found: {}", file_path));
    }

    // Read file with detailed error context
    let json_str = fs::read_to_string(file_path)
        .map_err(|e| anyhow::anyhow!("Failed to read JSON file '{}': {}", file_path, e))?;

    // Check if file is empty
    if json_str.trim().is_empty() {
        return Err(anyhow::anyhow!("JSON file is empty: {}", file_path));
    }

    // Parse JSON with error context
    let receipt: Receipt = serde_json::from_str(&json_str).map_err(|e| {
        anyhow::anyhow!(
            "Failed to parse JSON receipt from '{}': {}. Check JSON syntax and structure.",
            file_path,
            e
        )
    })?;

    Ok(receipt)
}
//NOTE: receipt must be of kind succinct sothat this works
// let opts = ProverOpts::default().with_receipt_kind(ReceiptKind::Succinct);
// let receipt_info = prover.prove_with_opts(prove_env, &elf_bytes, &opts)?;
// let receipt = &receipt_info.receipt;
// let receipt_json = serde_json::to_string_pretty(&receipt)?;
// std::fs::write("receipt.json", receipt_json)?;
pub fn stark_receipt_to_snark(stark_receipt: Receipt) -> Result<Vec<u8>> {
    let succinct_receipt = stark_receipt.inner.succinct().unwrap();
    let receipt = risc0_zkvm::recursion::identity_p254(&succinct_receipt).unwrap();
    let seal_bytes = receipt.get_seal_bytes();
    let seal = stark_to_snark(&seal_bytes)?.to_vec();

    Ok(seal)
}

pub fn main() {
    let rj = load_receipt_from_json("../../test_data/merkel_receipt/receipt.json").unwrap();

    let start = Instant::now();
    println!("Started coversion:");

    let seal = stark_receipt_to_snark(rj);

    print!("{:?}", seal);
    let duration = start.elapsed();
    println!("Time taken: {:?}", duration);
}
