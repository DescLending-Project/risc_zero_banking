use ethers::types::EIP1186ProofResponse;
use fetch_merkle::{UserHistory, UserHistoryProof};
use score_calculation::CreditInput;
// use serde::{Deserialize, Serialize};
// use serde_json::{Value, json};
use merkle_verifier_core::merkle_patricia::verify_eth_proof;
pub struct AllMerkleProofs {
    pub user_history_proof: UserHistoryProof,
    pub owned_accounts_merkle_proofs: Vec<EIP1186ProofResponse>,
}

// TODO: 1. Convert the types and call verify_eth_proof
pub async fn verify_all_merkle_proofs(all_merkle_proofs: AllMerkleProofs) {
    println!("{:?}", all_merkle_proofs.owned_accounts_merkle_proofs);
    println!("{:?}", all_merkle_proofs.user_history_proof);
    println!("yes");
}
