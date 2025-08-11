use anyhow::Result;
use risc0_zkvm::{Receipt, sha::Digestible};
use bincode;
use std::{env, fs};

fn main() -> Result<()> {
    // Get receipt path from command line args
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <receipt.bin>", args[0]);
        std::process::exit(1);
    }
    
    let receipt_path = &args[1];
    
    // Load and deserialize the receipt
    let receipt_bytes = fs::read(receipt_path)?;
    let receipt: Receipt = bincode::deserialize(&receipt_bytes)?;

    // Extract the ImageID
    let claim = receipt.claim()?.value().expect("Claim was pruned");
    let pre_state = claim.pre.value().expect("Pre-state was pruned");
    let image_id = pre_state.digest();
    
    // Print the ImageID as a Rust array
    println!("const IMAGE_ID: [u32; 8] = [");
    for (i, word) in image_id.as_words().iter().enumerate() {
        if i == image_id.as_words().len() - 1 {
            println!("    0x{:08x},", word);
        } else {
            println!("    0x{:08x},", word);
        }
    }
    println!("];");

    Ok(())
}