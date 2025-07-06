use eth_utils::{Node, setup_eth_provider};
// use ethereum_types::Address;
use anyhow::{Context, Result};
use ethers::{
    providers::Middleware,
    types::{Address, BlockId, BlockNumber, EIP1186ProofResponse, H256},
};
use fetch_merkle::{MerkleProofFetcher, UserHistory, UserHistoryProof};
use merkle_verifier_core::merkle_patricia::AccountMerkleProof;
use std::str::FromStr;

use crate::guest::{AllMerkleProofs, verify_all_merkle_proofs};

pub async fn fetch_all_merkle_proofs(
    contract_address: Address,
    user_address: Address,
    user_owned_addresses: Vec<Address>,
) -> Result<AllMerkleProofs> {
    let provider = setup_eth_provider(Node::Anvil).await.unwrap();
    // let block_number = provider.get_block(BlockNumber::Latest).await?.unwrap();
    // let user_history_proof = get_user_complete_history(&provider, user_address, contract_address).await;
    let block_id = BlockId::Number(BlockNumber::Latest);
    println!("{:?}", block_id);
    let fetcher = MerkleProofFetcher::new("http://localhost:8545", Some(provider)).unwrap();

    let block = BlockId::Number(BlockNumber::Latest);
    let user_history_proof = fetcher
        .fetch_complete_user_data(contract_address, user_address, block)
        .await
        .unwrap();
    let mut owned_accounts_merkle_proofs: Vec<AccountMerkleProof> = Vec::new();
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
    let user_address2 = Address::from_str("0x70997970C51812dc3A010C7d01b50e0d17dc79C8").unwrap();
    let user_owned_addresses: Vec<Address> = vec![user_address, user_address2];
    let all_merkle_proofs: AllMerkleProofs =
        fetch_all_merkle_proofs(contract_address, user_address, user_owned_addresses)
            .await
            .unwrap();
    // println!("+++++++++++++++++++++++++++++++++++++++++++++");
    // println!("all_merkle_proofs");
    // println!("{:?}", all_merkle_proofs.owned_accounts_merkle_proofs);
    // println!("{:?}", all_merkle_proofs.owned_accounts_merkle_proofs);
    // println!("+++++++++++++++++++++++++++++++++++++++++++++");

    verify_all_merkle_proofs(all_merkle_proofs).await;
}
