#![no_std]
#![no_main]
extern crate alloc;
use alloc::{string::String, vec::Vec, vec, format};
use alloc::string::ToString;
use risc0_zkvm::guest::env;
use risc0_zkvm::serde::from_slice;
use serde::{Deserialize, Serialize};
use ethereum_types::{H256, U256};
use alloy_sol_types::{SolValue};
use score_calculation::{
    CreditInput, PaymentHistory, TrustLevel, CreditScoreBreakdown,
    calculate_score
};
use defi_inputs_serializer::DefiProofOutput;

risc0_zkvm::guest::entry!(main);

const TRADFI_PROOF_IMAGE_ID: [u32; 8] = [
    0x7bd6867a,
    0xd6e5068c,
    0x66cc3bad,
    0x5bd072c9,
    0x4475f55a,
    0x6e0c7d62,
    0xb70bbab8,
    0x0cc197d3,
];

const DEFI_PROOF_IMAGE_ID: [u32; 8] = [
    0xe9147e8f, 0x388f434f, 0xb08b06f3, 0x1f864690, 
    0xe73209ac, 0xe9bd2ebb, 0x23740800, 0xe4a69d26,
];

const STATEROOT_PROOF_IMAGE_ID: [u32; 8] = [
    0xab8e6209, 0x1f2a7970, 0xacdd7e5d, 0xe74ca169,
    0x2ed86810, 0x9189b237, 0x8a8b2512, 0x9706dfbb,
];

#[derive(Debug, Serialize, Deserialize)]
struct VerificationOutput {
    is_valid: bool,
    server_name: String,
    score: Option<u64>,
    user_id_hash: Option<String>, 
    tradfi_date_timestamp: Option<u64>, 
    error: Option<String>,
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
    score: u64,
    server_name: String,
    state_root_provider: String,
    block_number: u64,
    user_id_hash: String,
    tradfi_date_timestamp: u64, 
    user_address: String,
    all_nullifiers: Vec<H256>,
    breakdown: CreditScoreBreakdown,
}

fn u256_to_u64(value: U256) -> u64 {
    if value > U256::from(u64::MAX) {
        u64::MAX
    } else {
        value.as_u64()
    }
}

fn u256_to_u128(value: U256) -> u128 {
    if value > U256::from(u128::MAX) {
        u128::MAX
    } else {
        value.as_u128()
    }
}

fn h160_to_string(address: ethereum_types::H160) -> String {
    format!("{:?}", address)
}

fn bytes32_vec_to_h256_vec(bytes_vec: Vec<[u8; 32]>) -> Vec<H256> {
    bytes_vec.into_iter().map(|bytes| H256::from(bytes)).collect()
}

fn parse_block_number(block_number_str: &Option<String>) -> u64 {
    block_number_str
        .as_ref()
        .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
        .unwrap_or(0)
}

fn validate_state_root_consistency(
    _defi_data: &DefiProofOutput,
    _stateroot_data: &StateRootOutput,
) -> bool {
    // State root validation temporarily disabled
    true
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

fn create_zero_score_hybrid(
    server_name: &str,
    tradfi_data: &VerificationOutput,
    defi_data: &DefiProofOutput,
    stateroot_data: &StateRootOutput,
) -> HybridCreditScore {
    HybridCreditScore {
        score: 0,
        server_name: server_name.to_string(),
        state_root_provider: stateroot_data.server_name.clone(),
        block_number: parse_block_number(&stateroot_data.block_number),
        user_id_hash: tradfi_data.user_id_hash.clone().unwrap_or_default(),
        tradfi_date_timestamp: tradfi_data.tradfi_date_timestamp.unwrap_or(0),
        user_address: h160_to_string(defi_data.user_address),
        all_nullifiers: bytes32_vec_to_h256_vec(defi_data.all_nullifiers.clone()),
        breakdown: create_zero_breakdown(),
    }
}

fn calculate_hybrid_score_with_lib(
    tradfi_data: &VerificationOutput,
    defi_data: &DefiProofOutput,
    stateroot_data: &StateRootOutput,
) -> HybridCreditScore {
    let current_block = parse_block_number(&stateroot_data.block_number);
    
    let payment_history = PaymentHistory {
        on_time_payments: u256_to_u64(defi_data.on_time_payments),
        liquidations: u256_to_u64(defi_data.liquidations),
    };
    
    let credit_input = CreditInput {
        first_interaction_timestamp: u256_to_u64(defi_data.first_interaction_timestamp),
        current_block,
        payment_history,
        total_eth_balance: u256_to_u128(defi_data.total_eth_balance),
        current_debt: u256_to_u128(defi_data.current_debt),
        tradify_credit_score: tradfi_data.score.map(|s| s as u16),
        trust_level: TrustLevel::Platinum,
    };

    let breakdown = match calculate_score(credit_input) {
        Ok(breakdown) => breakdown,
        Err(_) => {
            return HybridCreditScore {
                score: 0,
                server_name: "CALCULATION_FAILED".to_string(),
                state_root_provider: stateroot_data.server_name.clone(),
                block_number: current_block,
                user_id_hash: tradfi_data.user_id_hash.clone().unwrap_or_default(),
                tradfi_date_timestamp: tradfi_data.tradfi_date_timestamp.unwrap_or(0),
                user_address: h160_to_string(defi_data.user_address),
                all_nullifiers: bytes32_vec_to_h256_vec(defi_data.all_nullifiers.clone()),
                breakdown: create_zero_breakdown(),
            };
        }
    };

    HybridCreditScore {
        score: breakdown.final_score as u64,
        server_name: tradfi_data.server_name.clone(),
        state_root_provider: stateroot_data.server_name.clone(),
        block_number: current_block,
        user_id_hash: tradfi_data.user_id_hash.clone().unwrap_or_default(),
        tradfi_date_timestamp: tradfi_data.tradfi_date_timestamp.unwrap_or(0),
        user_address: h160_to_string(defi_data.user_address),
        all_nullifiers: bytes32_vec_to_h256_vec(defi_data.all_nullifiers.clone()),
        breakdown,
    }
}

use alloy_sol_types::{sol};
use alloy_sol_types::private::FixedBytes;

sol! {
    struct JournalData {
        uint64 score;
        string serverName;
        string stateRootProvider; 
        uint64 blockNumber;
        string userIdHash;          
        uint64 tradfiDateTimestamp;  
        string userAddress;
        bytes32[] allNullifiers;
    }
}

fn commit_hybrid_score(hybrid: &HybridCreditScore) {
    let nullifiers_fixed_bytes: Vec<FixedBytes<32>> = hybrid.all_nullifiers
        .iter()
        .map(|h| FixedBytes::<32>::from(h.0))
        .collect();

    let journal_struct = JournalData {
        score: hybrid.score,
        serverName: hybrid.server_name.clone(),
        stateRootProvider: hybrid.state_root_provider.clone(),
        blockNumber: hybrid.block_number,
        userIdHash: hybrid.user_id_hash.clone(),          
        tradfiDateTimestamp: hybrid.tradfi_date_timestamp, 
        userAddress: hybrid.user_address.clone(),
        allNullifiers: nullifiers_fixed_bytes,
    };
    
    let encoded_journal = journal_struct.abi_encode();
    env::commit_slice(&encoded_journal);
}

fn commit_zero_score() {
    let journal_struct = JournalData {
        score: 0u64,
        serverName: String::from(""),
        stateRootProvider: String::from(""),
        blockNumber: 0u64,
        userIdHash: String::from(""),        
        tradfiDateTimestamp: 0u64,           
        userAddress: String::from(""),
        allNullifiers: vec![],
    };
    
    let encoded_journal = journal_struct.abi_encode();
    env::commit_slice(&encoded_journal);
}

fn main() {
    // Read journal bytes from all three proofs
    let tradfi_journal_bytes: Vec<u8> = env::read();
    let defi_journal_bytes: Vec<u8> = env::read();
    let stateroot_journal_bytes: Vec<u8> = env::read();

    // Verify all three proofs
    if env::verify(TRADFI_PROOF_IMAGE_ID, &tradfi_journal_bytes).is_err() {
        commit_zero_score();
        return;
    }
    if env::verify(DEFI_PROOF_IMAGE_ID, &defi_journal_bytes).is_err() {
        commit_zero_score();
        return;
    }
    if env::verify(STATEROOT_PROOF_IMAGE_ID, &stateroot_journal_bytes).is_err() {
        commit_zero_score();
        return;
    }

    // Decode all verified journals
    let tradfi_data: VerificationOutput = match from_slice(&tradfi_journal_bytes) {
        Ok(data) => data,
        Err(_) => {
            commit_zero_score();
            return;
        }
    };
    let defi_data: DefiProofOutput = match from_slice(&defi_journal_bytes) {
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

    // Validate all proofs
    if !tradfi_data.is_valid {
        let hybrid_score = create_zero_score_hybrid("TRADFI_INVALID", &tradfi_data, &defi_data, &stateroot_data);
        commit_hybrid_score(&hybrid_score);
        return;
    }

    if !stateroot_data.is_valid {
        let hybrid_score = create_zero_score_hybrid("STATEROOT_INVALID", &tradfi_data, &defi_data, &stateroot_data);
        commit_hybrid_score(&hybrid_score);
        return;
    }

    if !validate_state_root_consistency(&defi_data, &stateroot_data) {
        let hybrid_score = create_zero_score_hybrid("STATEROOT_MISMATCH", &tradfi_data, &defi_data, &stateroot_data);
        commit_hybrid_score(&hybrid_score);
        return;
    }

    // Calculate and commit hybrid credit score
    let hybrid_score = calculate_hybrid_score_with_lib(&tradfi_data, &defi_data, &stateroot_data);
    commit_hybrid_score(&hybrid_score);
}