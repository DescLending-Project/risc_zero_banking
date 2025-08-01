#![no_std]
#![no_main]
extern crate alloc;
use alloc::{string::String, vec::Vec, vec};
use risc0_zkvm::guest::env;
use risc0_zkvm::serde::from_slice;
use serde::{Deserialize, Serialize};
use ethereum_types::{H256, U256};
use score_calculation::{
    CreditInput, PaymentHistory, TrustLevel, CreditScoreBreakdown,
    calculate_score
};

risc0_zkvm::guest::entry!(main);

const TRADFI_PROOF_IMAGE_ID: [u32; 8] = [
    0x720afb3a, 0xe6dfd539, 0x727f1629, 0xb9653d26,
    0x183da913, 0x168cb59c, 0xb70d0d1a, 0x063ce56b,
];
const ACCOUNT_PROOF_IMAGE_ID: [u32; 8] = [
    0xbb602230, 0x67951e01, 0xd4418e62, 0xe947bfa6,
    0x610d6021, 0x78a63884, 0x0e4d6fc6, 0x9fe342a1,
];
const STATEROOT_PROOF_IMAGE_ID: [u32; 8] = [
    0xab8e6209, 0x1f2a7970, 0xacdd7e5d, 0xe74ca169,
    0x2ed86810, 0x9189b237, 0x8a8b2512, 0x9706dfbb,
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

#[derive(Debug, Serialize, Deserialize)]
struct StateRootOutput {
    is_valid: bool,
    server_name: String,
    state_root: Option<String>,
    block_number: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct HybridCreditScore {
    score: u16, 
    server_name: String,
    state_root_provider: String,
    block_number: u64,
    breakdown: CreditScoreBreakdown,
}

fn main() {
    // 1) Read the journal bytes from all three proofs
    let tradfi_journal_bytes: Vec<u8> = env::read();
    let account_journal_bytes: Vec<u8> = env::read();
    let stateroot_journal_bytes: Vec<u8> = env::read();

    // 2) Verify all three proofs
    if let Err(_e) = env::verify(TRADFI_PROOF_IMAGE_ID, &tradfi_journal_bytes) {
        commit_zero_score();
        return;
    }
    if let Err(_e) = env::verify(ACCOUNT_PROOF_IMAGE_ID, &account_journal_bytes) {
        commit_zero_score();
        return;
    }
    if let Err(_e) = env::verify(STATEROOT_PROOF_IMAGE_ID, &stateroot_journal_bytes) {
        commit_zero_score();
        return;
    }

    // 3) Decode all verified journals
    let tradfi_data: VerificationOutput = match from_slice(&tradfi_journal_bytes) {
        Ok(data) => data,
        Err(_) => {
            commit_zero_score();
            return;
        }
    };
    let account_data: ProofOutput = match from_slice(&account_journal_bytes) {
        Ok(data) => data,
        Err(_) => {
            commit_zero_score();
            return;
        }
    };
    let stateroot_data: StateRootOutput = match from_slice(&stateroot_journal_bytes) {
        Ok(data) => data,
        Err(_) => {
            commit_zero_score();
            return;
        }
    };

    // 4) Validate all proofs and cross-verify state root
    if !tradfi_data.is_valid {
        let hybrid_score = HybridCreditScore {
            score: 0,
            server_name: tradfi_data.server_name.clone(),
            state_root_provider: stateroot_data.server_name.clone(),
            block_number: parse_block_number(&stateroot_data.block_number),
            breakdown: create_zero_breakdown(),
        };
        commit_hybrid_score(&hybrid_score);
        return;
    }

    if !stateroot_data.is_valid {
        let hybrid_score = HybridCreditScore {
            score: 0,
            server_name: tradfi_data.server_name.clone(),
            state_root_provider: stateroot_data.server_name.clone(),
            block_number: parse_block_number(&stateroot_data.block_number),
            breakdown: create_zero_breakdown(),
        };
        commit_hybrid_score(&hybrid_score);
        return;
    }

    if !validate_state_root_consistency(&account_data, &stateroot_data) {
        let hybrid_score = HybridCreditScore {
            score: 0,
            server_name: tradfi_data.server_name.clone(),
            state_root_provider: stateroot_data.server_name.clone(),
            block_number: parse_block_number(&stateroot_data.block_number),
            breakdown: create_zero_breakdown(),
        };
        commit_hybrid_score(&hybrid_score);
        return;
    }

    // 5) Calculate hybrid credit score using the library
    let hybrid_score = calculate_hybrid_score_with_lib(&tradfi_data, &account_data, &stateroot_data);

    // 6) Commit the result
    commit_hybrid_score(&hybrid_score);
}

fn parse_block_number(block_number_str: &Option<String>) -> u64 {
    block_number_str
        .as_ref()
        .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
        .unwrap_or(0)
}
//TODO
fn validate_state_root_consistency(
    account_data: &ProofOutput,
    stateroot_data: &StateRootOutput,
) -> bool {

    true 
}

fn calculate_hybrid_score_with_lib(
    tradfi_data: &VerificationOutput,
    account_data: &ProofOutput,
    stateroot_data: &StateRootOutput,
) -> HybridCreditScore {
   
    let current_block = parse_block_number(&stateroot_data.block_number);
    
    // Estimate account age in blocks based on nonce 
    let estimated_account_age_blocks = if let Some(nonce) = account_data.nonce {
        let nonce_value = nonce.as_u64();
        match nonce_value {
            0 => 2000,       
            1..=10 => 10000,  
            11..=50 => 50000,
            51..=100 => 200000, 
            _ => 500000,     
        }
    } else {
        0 
    };
    
    let first_interaction_block = current_block.saturating_sub(estimated_account_age_blocks);
    
    let credit_input = CreditInput {
        first_interaction_block,
        current_block,
        payment_history: PaymentHistory {
            on_time_payments: 0, // n/a yet
            liquidations: 0,
        },
        total_eth_balance: account_data.balance
            .map(|b| b.as_u128())
            .unwrap_or(0),
        current_debt: 0,
        tradify_credit_score: tradfi_data.score.map(|s| s as u16),
        trust_level: TrustLevel::Platinum,
    };

    let breakdown = calculate_score(credit_input)
        .expect("Robust library should never fail with valid inputs");

    HybridCreditScore {
        score: breakdown.final_score,
        server_name: tradfi_data.server_name.clone(),
        state_root_provider: stateroot_data.server_name.clone(),
        block_number: current_block,
        breakdown,
    }
}

fn create_zero_breakdown() -> CreditScoreBreakdown {
    CreditScoreBreakdown {
        length_of_history_score: 300,
        payment_history_score: 300,
        credit_utilization_score: 300,
        tradify_integration_score: 300,
        trust_factor_score: 300,
        final_score: 300,
    }
}

fn commit_hybrid_score(hybrid: &HybridCreditScore) {
    let mut data = [0u8; 128]; 
    
    let score_bytes = (hybrid.score as u64).to_le_bytes();
    data[0..8].copy_from_slice(&score_bytes);
    
    let server_bytes = hybrid.server_name.as_bytes();
    let server_copy_len = server_bytes.len().min(48);
    data[8..8 + server_copy_len].copy_from_slice(&server_bytes[..server_copy_len]);
    
    let provider_bytes = hybrid.state_root_provider.as_bytes();
    let provider_copy_len = provider_bytes.len().min(48);
    data[56..56 + provider_copy_len].copy_from_slice(&provider_bytes[..provider_copy_len]);
    
    let block_number_bytes = hybrid.block_number.to_le_bytes();
    data[104..112].copy_from_slice(&block_number_bytes);
    
    let data_vec = data.to_vec();
    env::commit(&data_vec);
}

fn commit_zero_score() {
    let data = vec![0u8; 128];
    env::commit(&data);
}