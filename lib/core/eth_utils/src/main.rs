// src/main.rs

use ethers::types::Address;
use std::fs;
use std::str::FromStr;

// Import from our library (replace "eth_signatures" with your actual crate name from Cargo.toml)
use eth_utils::*;

// Or import specific functions:
// use eth_signatures::{
//     sign_message,
//     verify_signature,
//     get_wallet_address,
//     sign_and_verify,
//     batch_sign_messages,
// };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example usage with Anvil accounts
    let private_key1 = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
    let private_key2 = "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d";
    let message1 = "Hello, Ethereum! This is a signed message.";
    let message2 = "Different message from second wallet";

    println!("=== Ethereum Signature Demo ===\n");

    // Test signing and verification
    println!("1. Basic signing and verification:");
    let signature1 = sign_message(private_key1, message1).await?;
    let wallet1_address = get_wallet_address(private_key1)?;
    let is_valid1 = verify_signature(message1, &signature1, wallet1_address)?;
    println!("Signature valid: {}\n", is_valid1);

    // Test with second wallet
    println!("2. Second wallet test:");
    let signature2 = sign_message(private_key2, message2).await?;
    let wallet2_address = get_wallet_address(private_key2)?;
    let is_valid2 = verify_signature(message2, &signature2, wallet2_address)?;
    println!("Signature valid: {}\n", is_valid2);

    // Test cross-verification (should fail)
    println!("3. Cross-verification test (should fail):");
    let cross_valid = verify_signature(message1, &signature2, wallet1_address)?;
    println!("Cross verification result: {}\n", cross_valid);

    // // Test one-shot sign and verify
    // println!("4. One-shot sign and verify:");
    // let one_shot_result = sign_and_verify(private_key1, "One-shot test message").await?;
    // println!("One-shot result: {}\n", one_shot_result);

    // Demonstrate batch signing
    println!("5. Batch signing:");
    let messages = vec!["Transaction #1", "Transaction #2", "Transaction #3"];

    let signatures = batch_sign_messages(private_key1, &messages).await?;
    println!("Successfully batch signed {} messages\n", signatures.len());

    // Show signature details
    println!("6. Signature details for first message:");
    let sig = &signatures[0];
    println!("  r: {:?}", sig.r);
    println!("  s: {:?}", sig.s);
    println!("  v: {}", sig.v);

    // Convert signature to bytes
    let sig_bytes = sig.to_vec();
    // println!("  Signature as bytes: 0x{}", hex::encode(&sig_bytes));
    println!("  Signature length: {} bytes", sig_bytes.len());

    println!("\n=== Demo Complete ===");
    println!("\n=== Generating signed msg ===");

    // Load the JSON file
    let json_content = fs::read_to_string("../../test_data/eth_utils/anvil_accounts.json")?;

    // Parse the JSON
    let accounts_data: AnvilAccountsData = serde_json::from_str(&json_content)?;
    let msg = "risc_zero_banking";

    println!("Loaded {} accounts from JSON", accounts_data.accounts.len());
    let private_keys: Vec<&str> = accounts_data
        .accounts
        .iter()
        .map(|acc| acc.private_key.as_str())
        .collect();
    let results = batch_sign_message(&msg, &private_keys).await.unwrap();
    let results_json = serde_json::to_string_pretty(&results)?;
    fs::write("./anvil_signatures_risc_zero_banking.json", results_json);

    println!("{:?}", results);

    Ok(())
}
