use eth_utils::{Node, setup_eth_provider};
use ethereum_types::{Address, U256};
// use ethereum_types::Address;
use anyhow::{Context, Ok, Result};
use ethers::{
    providers::Middleware,
    types::{BlockId, BlockNumber, EIP1186ProofResponse, H256},
};
use fetch_merkle::{AllMerkleProofs, MerkleProofFetcher, UserHistoryProof};
use loaders::loaders::{
    load_all_merkle_proofs, load_nullifiers, load_signatures, load_user_owned_addresses,
    save_all_merkle_proofs, save_nullifiers, save_signatures, save_user_owned_addresses,
};
use merkle_verifier_core::merkle_patricia::AccountMerkleProof;
use nullifier_verifier_core::nullifiers::{generate_all_nullifiers, verify_all_nullifiers};
use signature_verifier_core::signature_verifier::{generate_all_signatures, verify_all_signatures};
use std::str::FromStr;

use merkle_verifier_core::merkle_patricia::{
    AccountData, verify_account_proof, verify_all_account_proofs, verify_all_storage_proofs,
    verify_storage_proof,
};
#[tokio::test]
async fn defi_inputs_validation_test() {
    let message = "Block 2";
    let all_signatures = load_signatures("signatures.json").unwrap();
    let all_nullifiers = load_nullifiers("nullifiers.json").unwrap();
    let all_merkle_proofs = load_all_merkle_proofs("all_merkle_proofs.json").unwrap();
    let user_owned_addresses = load_user_owned_addresses("user_owned_addresses.json").unwrap();

    let trusted_state_root = all_merkle_proofs.user_history_proof.state_root;

    // 1. Verifying all owned account proofs and getting the total eth balance of all owned accounts
    let mut total_eth_balance: U256 = verify_all_account_proofs(
        &all_merkle_proofs.owned_accounts_merkle_proofs,
        &user_owned_addresses,
        &trusted_state_root,
    );

    // 2. Verifying merkle proof of lending contract to ensure that defi data came from our
    //    contract
    let contract_merkle_proof = all_merkle_proofs.user_history_proof.contract_merkle_proof;
    let contract = verify_account_proof(
        trusted_state_root.clone(),
        &contract_merkle_proof.address,
        &contract_merkle_proof.account_proof,
    )
    .unwrap()
    .unwrap();

    // 3. Verifying merklpe proofs of all storge slots of the user and retriving their values
    let user_history_data = verify_all_storage_proofs(
        &all_merkle_proofs.user_history_proof.storage_merkle_proofs,
        &contract.storage_root,
        &user_owned_addresses[0],
    );

    // 4. verify the signatures
    let signatures_valid = verify_all_signatures(message, &all_signatures, &user_owned_addresses);
    // 5. verify the nullifers
    let nullifiers_valid =
        verify_all_nullifiers(&all_nullifiers, &all_signatures, &user_owned_addresses);

    println!("\nTotal eth balance: {:?}", total_eth_balance);
    println!("Constract: {:?}", contract);
    println!("User history: {:?}", user_history_data);
    assert!(signatures_valid, "All signatures should be valid");
    assert!(nullifiers_valid, "All Nullifiers should be valid");
}

#[tokio::test]
async fn defi_inputs_fetch_and_validation_test() {
    //this stuff should ber provided as input to the proof
    let user_owned_private_keys: Vec<[u8; 32]> = vec![
        hex::decode("ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
            .unwrap()
            .try_into()
            .unwrap(), // Account 0
        hex::decode("59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d")
            .unwrap()
            .try_into()
            .unwrap(), // Account 1
        hex::decode("5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a")
            .unwrap()
            .try_into()
            .unwrap(), // Account 2
        hex::decode("7c852118294e51e653712a81e05800f419141751be58f605c371e15141b007a6")
            .unwrap()
            .try_into()
            .unwrap(), // Account 3
        hex::decode("47e179ec197488593b187f80a00eb0da91f1b9d0b13f8733639f19c30a34926a")
            .unwrap()
            .try_into()
            .unwrap(), // Account 4
    ];

    let user_owned_addresses = vec![
        Address::from_slice(&hex::decode("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266").unwrap()), // Account 0
        Address::from_slice(&hex::decode("70997970C51812dc3A010C7d01b50e0d17dc79C8").unwrap()), // Account 1
        Address::from_slice(&hex::decode("3C44CdDdB6a900fa2b585dd299e03d12FA4293BC").unwrap()), // Account 2
        Address::from_slice(&hex::decode("90F79bf6EB2c4f870365E785982E1f101E93b906").unwrap()), // Account 3
        Address::from_slice(&hex::decode("15d34AAf54267DB7D7c367839AAf71A00a2C6A65").unwrap()), // Account 4
    ];
    let contract_address = Address::from_str("0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512").unwrap();
    let user_address = user_owned_addresses[0];
    let message = "Block 2";
    let all_signatures = generate_all_signatures(user_owned_private_keys.clone(), message);
    let all_nullifiers = generate_all_nullifiers(&all_signatures, &user_owned_addresses);
    // Saving signatures and the nullfiers
    // save_user_owned_addresses(&user_owned_addresses, "user_owned_addresses.json");
    save_signatures(&all_signatures, "signatures.json").unwrap();
    save_nullifiers(&all_nullifiers, "nullifiers.json").unwrap();

    // THis is the host part: fetching all merkle proofs from the local anvil node
    let provider = setup_eth_provider(Node::Anvil).await.unwrap();
    let block_id = BlockId::Number(BlockNumber::Latest);
    println!("{:?}", block_id);

    let fetcher = MerkleProofFetcher::new("http://localhost:8545", Some(provider)).unwrap();
    let all_merkle_proofs = fetcher
        .fetch_all_merkle_proofs(
            contract_address,
            user_address,
            user_owned_addresses.clone(),
            block_id,
        )
        .await
        .unwrap(); // Save
    save_all_merkle_proofs(&all_merkle_proofs, "all_merkle_proofs.json").unwrap();

    // Load
    let all_merkle_proofs = load_all_merkle_proofs("all_merkle_proofs.json").unwrap();

    // the verification logic == stuff that needs to be runed in the guest

    let trusted_state_root = all_merkle_proofs.user_history_proof.state_root;
    // 1. Verifying all owned account proofs and getting the total eth balance of all owned accounts
    let mut total_eth_balance: U256 = verify_all_account_proofs(
        &all_merkle_proofs.owned_accounts_merkle_proofs,
        &user_owned_addresses,
        &trusted_state_root,
    );

    // 2. Verifying merkle proof of lending contract to ensure that defi data came from our
    //    contract
    let contract_merkle_proof = all_merkle_proofs.user_history_proof.contract_merkle_proof;
    let contract = verify_account_proof(
        trusted_state_root.clone(),
        &contract_merkle_proof.address,
        &contract_merkle_proof.account_proof,
    )
    .unwrap()
    .unwrap();

    // 3. Verifying merklpe proofs of all storge slots of the user and retriving their values
    let user_history_data = verify_all_storage_proofs(
        &all_merkle_proofs.user_history_proof.storage_merkle_proofs,
        &contract.storage_root,
        &user_owned_addresses[0],
    );

    // 4. verify the signatures
    let signatures_valid = verify_all_signatures(message, &all_signatures, &user_owned_addresses);
    // 5. verify the nullifers
    let nullifiers_valid =
        verify_all_nullifiers(&all_nullifiers, &all_signatures, &user_owned_addresses);

    println!("\nTotal eth balance: {:?}", total_eth_balance);
    println!("Constract: {:?}", contract);
    println!("User history: {:?}", user_history_data);
    assert!(signatures_valid, "All signatures should be valid");
    assert!(nullifiers_valid, "All Nullifiers should be valid");
}
