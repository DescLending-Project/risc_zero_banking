use crate::types::{ScoreGernerationInput, ScoreGernerationOutput};
use ethereum_types::{Address, H256, U256};
use ethers::types::{BlockId, BlockNumber};
use fetch_merkle::MerkleProofFetcher;
use merkle_verifier_core::merkle_patricia::{
    AccountMerkleProof, StorageMerkleProof, verify_account_proof, verify_all_account_proofs,
    verify_all_storage_proofs,
};
use nullifier_verifier_core::nullifiers::{generate_all_nullifiers, verify_all_nullifiers};
use score_calculation::{CreditInput, TrustLevel, calculate_credit_score};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use signature_verifier_core::signature_verifier::{generate_all_signatures, verify_all_signatures};
use std::str::FromStr;

fn main() {
    println!("Hello, world!");
}

pub async fn generate_score(
    ScoreGernerationInput {
        all_signatures,
        all_nullifiers,
        owned_accounts_addresses,
        contract_address,
        user_address,
        message,
        api_url,
        trusted_state_root,
        tradify_credit_score,
    }: ScoreGernerationInput,
) -> ScoreGernerationOutput {
    // THis is the host part: fetching all merkle proofs from the local anvil node
    let block_id = BlockId::Number(BlockNumber::Latest);
    println!("{:?}", block_id);

    let fetcher = MerkleProofFetcher::new(&api_url, None).unwrap();
    let all_merkle_proofs = fetcher
        .fetch_all_merkle_proofs(
            contract_address,
            user_address,
            owned_accounts_addresses.clone(),
            block_id,
        )
        .await
        .unwrap(); // Save
    // assert!(
    //     all_merkle_proofs.user_history_proof.state_root == trusted_state_root,
    //     "Trusted state_root mismach "
    // ); TODO: comment this back pls.
    // 1. Verifying all owned account proofs and getting the total eth balance of all owned accounts
    let total_wei_balance: U256 = verify_all_account_proofs(
        &all_merkle_proofs.owned_accounts_merkle_proofs,
        &owned_accounts_addresses,
        // &trusted_state_root, TODO: comment this back pls, and remove next line.
        &all_merkle_proofs.user_history_proof.state_root,
    );
    let eth_divisor = U256::exp10(18); // 10^18
    let total_eth_balance = total_wei_balance / eth_divisor;

    // 2. Verifying merkle proof of lending contract to ensure that defi data came from our
    //    contract
    let contract_merkle_proof = all_merkle_proofs.user_history_proof.contract_merkle_proof;
    let contract = verify_account_proof(
        // trusted_state_root.clone(),// &trusted_state_root, TODO: comment this back pls, and remove next line.
        all_merkle_proofs.user_history_proof.state_root.clone(),
        &contract_merkle_proof.address,
        &contract_merkle_proof.account_proof,
    )
    .unwrap()
    .unwrap();

    // 3. Verifying merklpe proofs of all storge slots of the user and retriving their values
    let user_history_data = verify_all_storage_proofs(
        &all_merkle_proofs.user_history_proof.storage_merkle_proofs,
        &contract.storage_root,
        &owned_accounts_addresses[0],
    );

    // 4. verify the signatures
    let signatures_valid =
        verify_all_signatures(&message, &all_signatures, &owned_accounts_addresses);
    // 5. verify the nullifers
    let nullifiers_valid =
        verify_all_nullifiers(&all_nullifiers, &all_signatures, &owned_accounts_addresses);

    let score = U256::from(850);
    assert!(signatures_valid, "All signatures should be valid");
    assert!(nullifiers_valid, "All Nullifiers should be valid");

    println!("\nTotal ETH balance: {:?}", total_eth_balance);
    println!("Constract: {:?}", contract);
    println!("User history: {:?}", user_history_data);
    println!("Calculated Score: {:?}", score);

    let credit_input = CreditInput {
        first_interaction_timestamp: user_history_data[0].into(),
        current_timestamp: U256::from(0),
        on_time_payments: user_history_data[1].into(),
        liquidations: user_history_data[2].into(),
        total_eth_balance,
        tradify_credit_score: Some(tradify_credit_score),
        trust_level: TrustLevel::TEE,
    };

    let score = calculate_credit_score(&credit_input);
    return ScoreGernerationOutput {
        all_nullifiers,
        contract_address,
        user_address,
        total_eth_balance,
        score,
    };
}
#[tokio::test]
async fn generate_score_test() {
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
    let api_url = "http://localhost:8545";
    let all_signatures = generate_all_signatures(user_owned_private_keys.clone(), message);
    let all_nullifiers = generate_all_nullifiers(&all_signatures, &user_owned_addresses);

    let block_id = BlockId::Number(BlockNumber::Latest);
    println!("{:?}", block_id);
    // for testing purposeses we fefch all_merkle proofs to make user the state_root is the same
    // in tee trusted_state_root is extracted from tlsn block info proof
    let fetcher = MerkleProofFetcher::new(&api_url, None).unwrap();
    let all_merkle_proofs = fetcher
        .fetch_all_merkle_proofs(
            contract_address,
            user_address,
            user_owned_addresses.clone(),
            block_id,
        )
        .await
        .unwrap(); // Save
    let trusted_state_root = all_merkle_proofs.user_history_proof.state_root;

    let score_gerneration_input: ScoreGernerationInput = ScoreGernerationInput {
        all_nullifiers,
        all_signatures,
        owned_accounts_addresses: user_owned_addresses,
        contract_address,
        user_address,
        message: message.to_string(),
        api_url: api_url.to_string(),
        trusted_state_root,
        tradify_credit_score: 850,
    };

    let res = generate_score(score_gerneration_input).await;
    println!("{:?}", res);
}

// Serializing and deserializing code
fn serialize_signatures<S>(signatures: &Vec<[u8; 65]>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let vec_of_vecs: Vec<Vec<u8>> = signatures.iter().map(|arr| arr.to_vec()).collect();
    vec_of_vecs.serialize(serializer)
}

fn deserialize_signatures<'de, D>(deserializer: D) -> Result<Vec<[u8; 65]>, D::Error>
where
    D: Deserializer<'de>,
{
    let vec_of_vecs: Vec<Vec<u8>> = Vec::deserialize(deserializer)?;
    vec_of_vecs
        .into_iter()
        .map(|v| {
            v.try_into()
                .map_err(|_| serde::de::Error::custom("Invalid signature length"))
        })
        .collect()
}

fn serialize_nullifiers<S>(nullifiers: &Vec<[u8; 32]>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let vec_of_vecs: Vec<Vec<u8>> = nullifiers.iter().map(|arr| arr.to_vec()).collect();
    vec_of_vecs.serialize(serializer)
}

fn deserialize_nullifiers<'de, D>(deserializer: D) -> Result<Vec<[u8; 32]>, D::Error>
where
    D: Deserializer<'de>,
{
    let vec_of_vecs: Vec<Vec<u8>> = Vec::deserialize(deserializer)?;
    vec_of_vecs
        .into_iter()
        .map(|v| {
            v.try_into()
                .map_err(|_| serde::de::Error::custom("Invalid signature length"))
        })
        .collect()
}
