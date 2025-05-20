#![no_std] // RISC Zero guest requires no_std
#![no_main] // RISC Zero guest r#![cfg_attr(test, no_main)]equires no_main
extern crate alloc;
use alloc::vec::Vec;
use ethereum_types::{H256, U256};
use risc0_zkvm::guest::env;
use serde::{Deserialize, Serialize};

use merkle_verifier_core::merkle_patricia::{verify_eth_proof, AccountData};

// Input structure
#[derive(Deserialize, Serialize)]
struct ProofInput {
    state_root: [u8; 32],
    address: [u8; 20],
    slot_key: Option<[u8; 32]>,
    account_proof: Vec<Vec<u8>>,
    storage_proof: Option<Vec<Vec<u8>>>,
}

// Output structure
#[derive(Serialize)]
struct ProofOutput {
    exists: bool,
    nonce: Option<U256>,
    balance: Option<U256>,
    storage_root: Option<H256>,
    code_hash: Option<H256>,
    storage_value: Option<U256>,
}

risc0_zkvm::guest::entry!(main);

// NOTE: For now we are returning the whole decoded information about the account.
// - inputs and outpust will change soon
// - this is only suposed to show that the eth merkel proof verification works properly
// TODO(likely):
// 1. State root verification (hopfully state_root hash  can be signed by alchemy but this need in general some  investigation )
// 2. Allowing for multiple account_proofs
// 3. Adding the balances together and returning them as result
pub fn main() {
    // Read the proof input
    let input: ProofInput = env::read();

    // Convert the state root to H256
    let state_root = H256::from_slice(&input.state_root);

    // Verify the proof
    let result = verify_eth_proof(
        state_root,
        input.address,
        input.slot_key,
        input.account_proof,
        input.storage_proof,
    );

    // Process the result
    let output = match result {
        Ok((Some(account), storage_value)) => ProofOutput {
            exists: true,
            nonce: Some(account.nonce),
            balance: Some(account.balance),
            storage_root: Some(account.storage_root),
            code_hash: Some(account.code_hash),
            storage_value,
        },
        Ok((Some(account), _)) => ProofOutput {
            exists: true,
            nonce: Some(account.nonce),
            balance: Some(account.balance),
            storage_root: Some(account.storage_root),
            code_hash: Some(account.code_hash),
            storage_value: None,
        },
        Ok((None, _)) => ProofOutput {
            exists: false,
            nonce: None,
            balance: None,
            storage_root: None,
            code_hash: None,
            storage_value: None,
        },
        Err(_) => ProofOutput {
            exists: false,
            nonce: None,
            balance: None,
            storage_root: None,
            code_hash: None,
            storage_value: None,
        },
    };

    // Commit the result
    env::commit(&output);
}
