#![no_std]
#![no_main]
extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;
use risc0_zkvm::guest::env;
use serde::{Deserialize, Serialize};
use ethereum_types::{H256, U256};

risc0_zkvm::guest::entry!(main);

const TRADFI_PROOF_IMAGE_ID: [u32; 8] = [
    0xa959aab4,
    0x03bb769a,
    0x559fc152,
    0xcbdcb7aa,
    0x7682affb,
    0x31a587a0,
    0x45eea34f,
    0xe98fa3b0,
];


const ACCOUNT_PROOF_IMAGE_ID: [u32; 8] = [
    0xbb602230,
    0x67951e01,
    0xd4418e62,
    0xe947bfa6,
    0x610d6021,
    0x78a63884,
    0x0e4d6fc6,
    0x9fe342a1,
];


// Input structures 
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

// Output structure for the hybrid credit score - minimal on-chain data
#[derive(Debug, Serialize, Deserialize)]
struct HybridCreditScore {
    score: u64,           // Final hybrid score (0-850)
    server_name: String,  // For on-chain server verification
}

fn main() {
    // 1) Read the journal bytes from both proofs
    let first_journal_bytes: Vec<u8> = env::read();
    let second_journal_bytes: Vec<u8> = env::read();
    
    // Debug: Check journal sizes
    if first_journal_bytes.is_empty() {
        panic!("First journal is empty!");
    }
    if second_journal_bytes.is_empty() {
        panic!("Second journal is empty!");
    }
    
    // 2) Verify both proofs with better error handling
    if let Err(e) = env::verify(TRADFI_PROOF_IMAGE_ID, &first_journal_bytes) {
        // Create error output instead of panicking
        let error_output = HybridCreditScore {
            score: 0,
            server_name: "TRADFI_VERIFICATION_FAILED".into(),
        };
        env::commit(&error_output);
        return;
    }
    
    if let Err(e) = env::verify(ACCOUNT_PROOF_IMAGE_ID, &second_journal_bytes) {
        // Create error output instead of panicking
        let error_output = HybridCreditScore {
            score: 0,
            server_name: "ACCOUNT_VERIFICATION_FAILED".into(),
        };
        env::commit(&error_output);
        return;
    }
    
    // 3) Decode the verified journals with error handling
    let tradfi_data: VerificationOutput = match risc0_zkvm::serde::from_slice(&first_journal_bytes) {
        Ok(data) => data,
        Err(_) => {
            let error_output = HybridCreditScore {
                score: 0,
                server_name: "TRADFI_DECODE_FAILED".into(),
            };
            env::commit(&error_output);
            return;
        }
    };
    
    let defi_data: ProofOutput = match risc0_zkvm::serde::from_slice(&second_journal_bytes) {
        Ok(data) => data,
        Err(_) => {
            let error_output = HybridCreditScore {
                score: 0,
                server_name: "ACCOUNT_DECODE_FAILED".into(),
            };
            env::commit(&error_output);
            return;
        }
    };
    
    // 4) Validate TradFi score is valid
    if !tradfi_data.is_valid {
        // If TradFi score is invalid, output error with score 0
        let error_output = HybridCreditScore {
            score: 0,
            server_name: tradfi_data.server_name,
        };
        env::commit(&error_output);
        return;
    }
    
    // 5) Calculate hybrid credit score
    let hybrid_score = calculate_hybrid_credit_score(&tradfi_data, &defi_data);
    
    // 6) Commit the result
    env::commit(&hybrid_score);
}

fn calculate_hybrid_credit_score(
    tradfi_data: &VerificationOutput, 
    defi_data: &ProofOutput
) -> HybridCreditScore {
    
    // Extract TradFi score (0-850 range typically)
    let tradfi_score = tradfi_data.score.unwrap_or(0);
    
    // Calculate DeFi score components if account exists
    let defi_score = if defi_data.exists {
        calculate_defi_score(defi_data)
    } else {
        0
    };
    
    // All the detailed checks happen here but don't get exposed:
    
    // Check if account is a smart contract (affects scoring internally)
    let is_contract = defi_data.code_hash
        .map(|hash| {
            let empty_code_hash = H256::from([
                0xc5, 0xd2, 0x46, 0x01, 0x86, 0xf7, 0x23, 0x3c,
                0x92, 0x7e, 0x7d, 0xb2, 0xdc, 0xc7, 0x03, 0xc0,
                0xe5, 0x00, 0xb6, 0x53, 0xca, 0x82, 0x27, 0x3b,
                0x7b, 0xfa, 0xd8, 0x04, 0x5d, 0x85, 0xa4, 0x70
            ]);
            hash != empty_code_hash
        })
        .unwrap_or(false);
    
    // Apply contract bonus/penalty internally
    let mut final_defi_score = defi_score;
    if is_contract {
        // Smart contracts might get slight penalty for complexity/risk
        final_defi_score = (final_defi_score as f64 * 0.95) as u64;
    }
    
    // Calculate hybrid score using weighted average
    // TradFi: 60% weight, DeFi: 40% weight
    let weighted_tradfi = (tradfi_score as f64 * 0.6) as u64;
    let weighted_defi = (final_defi_score as f64 * 0.4) as u64;
    let hybrid_score = weighted_tradfi + weighted_defi;
    
    // Cap the score at 850 (FICO range)
    let final_score = if hybrid_score > 850 { 850 } else { hybrid_score };
    
    // Only return the minimal data needed on-chain
    HybridCreditScore {
        score: final_score,
        server_name: tradfi_data.server_name.clone(),
    }
}

fn calculate_defi_score(defi_data: &ProofOutput) -> u64 {
    let mut score = 0u64;
    
    // 1. Account Age/Activity (nonce) - max 200 points
    if let Some(nonce) = defi_data.nonce {
        let nonce_score = if nonce > U256::from(1000) {
            200
        } else if nonce > U256::from(100) {
            150
        } else if nonce > U256::from(10) {
            100
        } else if nonce > U256::from(1) {
            50
        } else {
            10 // At least some activity
        };
        score += nonce_score;
    }
    
    // 2. ETH Balance - max 250 points (30% weight)
    if let Some(balance) = defi_data.balance {
        // Convert balance to ETH (approximate)
        let eth_balance = balance / U256::exp10(18); // Wei to ETH conversion
        
        let balance_score = if eth_balance >= U256::from(100) {
            250 // 100+ ETH
        } else if eth_balance >= U256::from(10) {
            200 // 10+ ETH
        } else if eth_balance >= U256::from(1) {
            150 // 1+ ETH
        } else if eth_balance >= U256::exp10(17) { // 0.1 ETH
            100
        } else if eth_balance >= U256::exp10(16) { // 0.01 ETH
            50
        } else if balance > U256::zero() {
            25 // Some balance
        } else {
            0 // No balance
        };
        score += balance_score;
    }
    
    // 3. Account existence and validity - max 100 points
    score += 100; // Base points for having a valid account
    
    // 4. Storage root complexity (indicates DeFi interaction) - max 100 points
    if let Some(_storage_root) = defi_data.storage_root {
        // If storage root is not empty, likely has interacted with contracts
        score += 50;
    }
    
    // Cap at 850 to match TradFi score range
    if score > 850 {
        score = 850;
    }
    
    score
}