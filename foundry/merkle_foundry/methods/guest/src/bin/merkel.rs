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
struct ProofPrivateInput {
    state_root: [u8; 32],
    address: [u8; 20],
    slot_key: Option<[u8; 32]>,
    account_proof: Vec<Vec<u8>>,
    storage_proof: Option<Vec<Vec<u8>>>,
}

#[derive(Deserialize, Serialize)]
struct ProofPublicInput {
    address: [u8; 20],
}

// Output structure
#[derive(Serialize)]
struct ProofOutput {
    address: Option<[u8; 20]>,
    balance: Option<U256>,
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
    let public_input: ProofPublicInput = env::read();
    let private_input: ProofPrivateInput = env::read();

    // Convert the state root to H256
    let state_root = H256::from_slice(&private_input.state_root);

    // Verify the proof
    let result = verify_eth_proof(
        state_root,
        private_input.address,
        private_input.slot_key,
        private_input.account_proof,
        private_input.storage_proof,
    );

    // Process the result
    let output = match result {
        Ok((Some(account), storage_value)) => ProofOutput {
            address: Some(public_input.address),
            balance: Some(account.balance),
        },
        Ok((Some(account), _)) => ProofOutput {
            address: Some(public_input.address),
            balance: Some(account.balance),
        },
        Ok((None, _)) => ProofOutput {
            address: None,
            balance: None,
        },
        Err(_) => ProofOutput {
            address: None,
            balance: None,
        },
    };

    // Commit the result
    env::commit(&output);
}
