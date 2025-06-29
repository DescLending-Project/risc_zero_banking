use eth_utils::{Node, setup_eth_provider};
// use ethereum_types::Address;
use anyhow::{Context, Result};
use ethers::types::{Address, BlockId, BlockNumber, EIP1186ProofResponse, H256};
use fetch_merkle::{MerkleProofFetcher, UserHistory, UserHistoryProof};
use score_calculation::CreditInput;
use std::{str::FromStr, vec};
// use serde::{Deserialize, Serialize};
// use serde_json::{Value, json};

use crate::guest::{AllMerkleProofs, verify_all_merkle_proofs};

pub async fn fetch_all_merkle_proofs(
    contract_address: Address,
    user_address: Address,
    user_owned_addresses: Vec<Address>,
) -> Result<AllMerkleProofs> {
    let provider = setup_eth_provider(Node::Anvil).await.unwrap();
    // let user_history_proof = get_user_complete_history(&provider, user_address, contract_address).await;
    let block_id = BlockId::Number(BlockNumber::Latest);
    println!("{:?}", block_id);
    let fetcher = MerkleProofFetcher::new("http://localhost:8545", Some(provider)).unwrap();

    let user_history_proof = fetcher
        .fetch_complete_user_data(contract_address, user_address)
        .await
        .unwrap();
    let mut owned_accounts_merkle_proofs: Vec<EIP1186ProofResponse> = Vec::new();
    for owned_addr in user_owned_addresses {
        let owned_account_merkle_proof = fetcher
            .fetch_account_merkle_proof(owned_addr, block_id)
            .await
            .unwrap();
        owned_accounts_merkle_proofs.push(owned_account_merkle_proof);
    }

    return Ok(AllMerkleProofs {
        user_history_proof,
        owned_accounts_merkle_proofs,
    });
}

#[tokio::test]
async fn fetch_and_verify_all_merkle_proofs_test() {
    let fetcher = MerkleProofFetcher::new("http://localhost:8545", None).unwrap();
    let contract_address = Address::from_str("0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512").unwrap();
    let user_address = Address::from_str("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266").unwrap();
    let user_owned_addresses: Vec<Address> = vec![user_address];
    let all_merkle_proofs: AllMerkleProofs =
        fetch_all_merkle_proofs(contract_address, user_address, user_owned_addresses)
            .await
            .unwrap();
    println!("{:?}", all_merkle_proofs.owned_accounts_merkle_proofs);
    println!("{:?}", all_merkle_proofs.owned_accounts_merkle_proofs);

    verify_all_merkle_proofs(all_merkle_proofs).await;
}
