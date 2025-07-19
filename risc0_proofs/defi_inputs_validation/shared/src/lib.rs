use ethereum_types::{Address, H256, U256};
use merkle_verifier_core::merkle_patricia::{AccountMerkleProof, StorageMerkleProof};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
// Wrapper type for [u8; 32] that implements Serialize/Deserialize

#[derive(Serialize, Deserialize)]
pub struct DefiProofInput {
    #[serde(
        serialize_with = "serialize_signatures",
        deserialize_with = "deserialize_signatures"
    )]
    pub all_signatures: Vec<[u8; 65]>,
    #[serde(
        serialize_with = "serialize_nullifiers",
        deserialize_with = "deserialize_nullifiers"
    )]
    pub all_nullifiers: Vec<[u8; 32]>,
    pub owned_accounts_addresses: Vec<Address>,
    pub owned_accounts_merkle_proofs: Vec<AccountMerkleProof>,
    pub storage_merkle_proofs: Vec<StorageMerkleProof>,
    pub contract_merkle_proof: AccountMerkleProof,
    pub contract_address: Address,
    pub user_address: Address,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DefiProofOutput {
    #[serde(
        serialize_with = "serialize_nullifiers",
        deserialize_with = "deserialize_nullifiers"
    )]
    pub all_nullifiers: Vec<[u8; 32]>,
    pub contract_address: Address,
    pub user_address: Address,
    pub message: String,
    pub total_eth_balance: U256,
    // Defi Payment history
    pub first_interaction_timestamp: U256,
    pub liquidations: U256,
    pub on_time_payments: U256,
    pub current_debt: U256,
}

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
