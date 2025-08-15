#![no_std]
#![no_main]
extern crate alloc;
use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use alloy_sol_types::SolValue;
use defi_inputs_serializer::DefiProofOutput;
use ethereum_types::{Address, H256, U256};
use risc0_zkvm::guest::env;
use risc0_zkvm::serde::from_slice;
use score_calculation::{calculate_credit_score, validate_input, CreditInput, TrustLevel};
use serde::{Deserialize, Serialize};

risc0_zkvm::guest::entry!(main);

const TRADFI_PROOF_IMAGE_ID: [u32; 8] = [
    0xc8b811ba,
    0xf3d515af,
    0x72224130,
    0xc07ab2a3,
    0xcab6ee6c,
    0x11738591,
    0x7c386aca,
    0xf32fa067,
];

const DEFI_PROOF_IMAGE_ID: [u32; 8] = [
    0xf98d9d31,
    0x5f239735,
    0x34780687,
    0x46ecb8a9,
    0xefeea591,
    0xaf482d49,
    0x17a57eb3,
    0xad7478b9,
];


const STATEROOT_PROOF_IMAGE_ID: [u32; 8] = [
    0xf22863eb,
    0x4e780014,
    0x5f5c09a8,
    0xafef2f7f,
    0x2f9668d8,
    0x4e9fd747,
    0x50f92ea4,
    0x8f1722e8,
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
    tradfi_nullifier: H256,
    tradfi_date_timestamp: u64,
    user_address: Address,
    contract_address: Address,
    all_nullifiers: Vec<H256>,
}

fn bytes32_vec_to_h256_vec(bytes_vec: Vec<[u8; 32]>) -> Vec<H256> {
    bytes_vec
        .into_iter()
        .map(|bytes| H256::from(bytes))
        .collect()
}

fn parse_block_number(block_number_str: &Option<String>) -> Result<u64, String> {
    match block_number_str {
        Some(s) => u64::from_str_radix(s.trim_start_matches("0x"), 16)
            .map_err(|_| format!("Invalid block number format: {}", s)),
        None => Err("Block number not provided".to_string()),
    }
}

fn hex_string_to_h256(hex_str: &str) -> Result<H256, String> {
    let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);

    if hex_str.len() > 64 {
        return Err("Hex string too long for H256".to_string());
    }

    let padded = format!("{:0<64}", hex_str);

    match hex::decode(&padded) {
        Ok(bytes) if bytes.len() == 32 => {
            let mut array = [0u8; 32];
            array.copy_from_slice(&bytes);
            Ok(H256::from(array))
        }
        Ok(_) => Err("Invalid hex string length".to_string()),
        Err(e) => Err(format!("Failed to decode hex: {}", e)),
    }
}

fn validate_state_root_consistency(
    defi_data: &DefiProofOutput,
    stateroot_data: &StateRootOutput,
) -> Result<(), String> {
    let stateroot_from_provider = stateroot_data
        .state_root
        .as_ref()
        .ok_or("State root not provided by state root proof")?;

    let defi_state_root_hex = format!("0x{:x}", defi_data.trusted_state_root);

    let normalized_provider = stateroot_from_provider
        .trim_start_matches("0x")
        .to_lowercase();
    let normalized_defi = defi_state_root_hex.trim_start_matches("0x").to_lowercase();

    if normalized_provider != normalized_defi {
        return Err(format!(
            "State root mismatch: provider={}, defi={}",
            normalized_provider, normalized_defi
        ));
    }

    Ok(())
}

fn validate_tradfi_data(tradfi_data: &VerificationOutput) -> Result<(), String> {
    if !tradfi_data.is_valid {
        return Err(format!(
            "TradFi verification failed: {:?}",
            tradfi_data.error
        ));
    }

    if tradfi_data.score.is_none() {
        return Err("TradFi score not provided".to_string());
    }

    if tradfi_data.user_id_hash.is_none() {
        return Err("TradFi user ID hash not provided".to_string());
    }

    Ok(())
}

fn validate_stateroot_data(stateroot_data: &StateRootOutput) -> Result<(), String> {
    if !stateroot_data.is_valid {
        return Err(format!(
            "State root verification failed: {:?}",
            stateroot_data.error
        ));
    }

    if stateroot_data.state_root.is_none() {
        return Err("State root not provided".to_string());
    }

    if stateroot_data.block_number.is_none() {
        return Err("Block number not provided".to_string());
    }

    Ok(())
}

fn calculate_hybrid_score_with_lib(
    tradfi_data: &VerificationOutput,
    defi_data: &DefiProofOutput,
    stateroot_data: &StateRootOutput,
) -> Result<HybridCreditScore, String> {
    let current_block = parse_block_number(&stateroot_data.block_number)?;

    // Use tradfi_date_timestamp as the current timestamp for score calculation
    let current_timestamp = tradfi_data
        .tradfi_date_timestamp
        .ok_or("TradFi date timestamp is required for score calculation")?;

    // Convert TradFi score from u64 to u16 (as the lib uses u16)
    let tradfi_score = tradfi_data.score.map(|s| {
        if s > 850 {
            850u16
        } else if s < 300 {
            300u16
        } else {
            s as u16
        }
    });

    // Create credit input using the library's expected structure
    let credit_input = CreditInput {
        first_interaction_timestamp: defi_data.first_interaction_timestamp,
        current_timestamp: U256::from(current_timestamp),
        on_time_payments: defi_data.on_time_payments,
        liquidations: defi_data.liquidations,
        total_eth_balance: defi_data.total_eth_balance,
        tradify_credit_score: tradfi_score,
        trust_level: TrustLevel::RiscZero, // Using RiscZero since we're in a RISC Zero environment
    };

    // Validate input using the library's validation function
    validate_input(&credit_input).map_err(|e| format!("Credit input validation failed: {}", e))?;

    let final_score = calculate_credit_score(&credit_input);

    let tradfi_nullifier = hex_string_to_h256(
        tradfi_data
            .user_id_hash
            .as_ref()
            .ok_or("TradFi user ID hash missing")?,
    )?;

    Ok(HybridCreditScore {
        score: final_score as u64, //back to u64
        server_name: tradfi_data.server_name.clone(),
        state_root_provider: stateroot_data.server_name.clone(),
        block_number: current_block,
        tradfi_nullifier,
        tradfi_date_timestamp: tradfi_data.tradfi_date_timestamp.unwrap_or(0),
        user_address: defi_data.user_address,
        contract_address: defi_data.contract_address,
        all_nullifiers: bytes32_vec_to_h256_vec(defi_data.all_nullifiers.clone()),
    })
}

use alloy_sol_types::private::FixedBytes;
use alloy_sol_types::sol;

sol! {
    struct JournalData {
        uint64 score;
        string serverName;
        string stateRootProvider;
        uint64 blockNumber;
        bytes32 tradfiNullifier;
        uint64 tradfiDateTimestamp;
        address userAddress;
        address contractAddress;
        bytes32[] allNullifiers;
    }
}

fn commit_hybrid_score(hybrid: &HybridCreditScore) -> Result<(), String> {
    let nullifiers_fixed_bytes: Vec<FixedBytes<32>> = hybrid
        .all_nullifiers
        .iter()
        .map(|h| FixedBytes::<32>::from(h.0))
        .collect();

    // Convert Ethereum Address to alloy address format
    let user_address_alloy = alloy_sol_types::private::Address::from(hybrid.user_address.0);
    let contract_address_alloy = alloy_sol_types::private::Address::from(hybrid.contract_address.0);

    let journal_struct = JournalData {
        score: hybrid.score,
        serverName: hybrid.server_name.clone(),
        stateRootProvider: hybrid.state_root_provider.clone(),
        blockNumber: hybrid.block_number,
        tradfiNullifier: FixedBytes::<32>::from(hybrid.tradfi_nullifier.0),
        tradfiDateTimestamp: hybrid.tradfi_date_timestamp,
        userAddress: user_address_alloy,
        contractAddress: contract_address_alloy,
        allNullifiers: nullifiers_fixed_bytes,
    };

    let encoded_journal = journal_struct.abi_encode();
    env::commit_slice(&encoded_journal);
    Ok(())
}

fn main() {
    let start = env::cycle_count();
    // Read journal bytes from all three proofs
    let tradfi_journal_bytes: Vec<u8> = env::read();
    let defi_journal_bytes: Vec<u8> = env::read();
    let stateroot_journal_bytes: Vec<u8> = env::read();

    // Verify all three proofs
    env::verify(TRADFI_PROOF_IMAGE_ID, &tradfi_journal_bytes)
        .expect("TradFi proof verification failed");

    env::verify(DEFI_PROOF_IMAGE_ID, &defi_journal_bytes).expect("DeFi proof verification failed");

    env::verify(STATEROOT_PROOF_IMAGE_ID, &stateroot_journal_bytes)
        .expect("State root proof verification failed");

    // Decode all verified journals
    let tradfi_data: VerificationOutput =
        from_slice(&tradfi_journal_bytes).expect("Failed to deserialize TradFi data");

    let defi_data: DefiProofOutput =
        from_slice(&defi_journal_bytes).expect("Failed to deserialize DeFi data");

    let stateroot_data: StateRootOutput =
        from_slice(&stateroot_journal_bytes).expect("Failed to deserialize state root data");

    // Validate all data integrity
    validate_tradfi_data(&tradfi_data).expect("TradFi data validation failed");

    validate_stateroot_data(&stateroot_data).expect("State root data validation failed");

    // Validate state root consistency between proofs
    validate_state_root_consistency(&defi_data, &stateroot_data)
        .expect("State root consistency validation failed");

    // Calculate hybrid credit score using the proper library
    let hybrid_score = calculate_hybrid_score_with_lib(&tradfi_data, &defi_data, &stateroot_data)
        .expect("Failed to calculate hybrid credit score");

    // Commit the final score
    commit_hybrid_score(&hybrid_score).expect("Failed to commit hybrid score");

    let total_cycles = env::cycle_count() - start;
    env::log(&format!("{}: Total Cycyles", total_cycles));
}