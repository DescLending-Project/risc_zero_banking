#![no_std]
#![no_main]

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

use risc0_zkvm::guest::env;
use serde::{Deserialize, Serialize};
use ethereum_types::{H256, U256};

risc0_zkvm::guest::entry!(main);

// Proof image IDs
const TRADFI_TLSN_PROOF_IMAGE_ID: [u32; 8] = [
    0x81c6f5d0,
    0x702b3a37,
    0x3ce771fe,
    0xbb63581e,
    0xd62fbd6f,
    0xf427e418,
    0x2bb82714,
    0x4e4a4c4c,
];

const ETH_ACCOUNT_PROOF_IMAGE_ID: [u32; 8] = [
    0xb083f461,
    0xf1de5891,
    0x87ceac04,
    0xa9eb2c0f,
    0xd7c99b8c,
    0x80f4ed3f,
    0x3d45963c,
    0x8b9606bf,
];

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

fn main() {
    // 1) Read the journal bytes
    let first_journal: Vec<u8> = env::read();
    let second_journal: Vec<u8> = env::read();

    // 2) Verify both proofs
    env::verify(TRADFI_TLSN_PROOF_IMAGE_ID, &first_journal)
        .expect("❌ TLSN proof verification failed");
    env::verify(ETH_ACCOUNT_PROOF_IMAGE_ID, &second_journal)
        .expect("❌ Ethereum account proof verification failed");

    // 3) Decode
    let tradfi: VerificationOutput =
        risc0_zkvm::serde::from_slice(&first_journal).expect("Failed to decode TLSN data");
    let defi: ProofOutput =
        risc0_zkvm::serde::from_slice(&second_journal).expect("Failed to decode Ethereum data");

    // 4) If TradFi invalid, commit zero score
    if !tradfi.is_valid {
        let out = HybridCreditScore {
            score: 0,
            server_name: tradfi.server_name,
        };
        env::commit(&out);
        return;
    }

    // 5) Compute hybrid and commit
    let hybrid = calculate_hybrid_credit_score(&tradfi, &defi);
    env::commit(&hybrid);
}

fn calculate_hybrid_credit_score(
    tradfi: &VerificationOutput,
    defi: &ProofOutput,
) -> HybridCreditScore {
    let tradfi_score = tradfi.score.unwrap_or(0);
    let mut defi_score = if defi.exists {
        calculate_defi_score(defi)
    } else {
        0
    };

    // contract penalty
    if let Some(code_hash) = defi.code_hash {
        let empty = H256::from([
            0xc5, 0xd2, 0x46, 0x01, 0x86, 0xf7, 0x23, 0x3c,
            0x92, 0x7e, 0x7d, 0xb2, 0xdc, 0xc7, 0x03, 0xc0,
            0xe5, 0x00, 0xb6, 0x53, 0xca, 0x82, 0x27, 0x3b,
            0x7b, 0xfa, 0xd8, 0x04, 0x5d, 0x85, 0xa4, 0x70,
        ]);
        if code_hash != empty {
            // slight penalty
            defi_score = (defi_score as f64 * 0.95) as u64;
        }
    }

    let weighted_tradfi = (tradfi_score as f64 * 0.6) as u64;
    let weighted_defi = (defi_score as f64 * 0.4) as u64;
    let total = weighted_tradfi + weighted_defi;

    HybridCreditScore {
        score: total,
        server_name: tradfi.server_name.clone(),
    }
}

fn calculate_defi_score(defi: &ProofOutput) -> u64 {
    let mut score = 0u64;

    // 1) Nonce activity
    if let Some(n) = defi.nonce {
        score += if n > U256::from(1000) {
            200
        } else if n > U256::from(100) {
            150
        } else if n > U256::from(10) {
            100
        } else if n > U256::from(1) {
            50
        } else {
            10
        };
    }

    // 2) ETH balance
    if let Some(bal) = defi.balance {
        let eth = bal / U256::exp10(18);
        score += if eth >= U256::from(100) {
            250
        } else if eth >= U256::from(10) {
            200
        } else if eth >= U256::from(1) {
            150
        } else if bal >= U256::exp10(17) {
            100
        } else if bal >= U256::exp10(16) {
            50
        } else if bal > U256::zero() {
            25
        } else {
            0
        };
    }

    // 3) Existence
    score += 100;

    // 4) Storage root
    if defi.storage_root.is_some() {
        score += 50;
    }

    // Cap at 850
    if score > 850 {
        score = 850;
    }

    score
}
