use ethereum_types::{Address, H256, U256};
use k256::ecdsa::{RecoveryId, Signature};
use merkle_verifier_core::merkle_patricia::{AccountMerkleProof, StorageMerkleProof};
use serde::de::{self, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

// Wrapper type for [u8; 32] that implements Serialize/Deserialize

#[derive(Serialize, Deserialize)]
pub struct DefiProofInput {
    #[serde(
        serialize_with = "serialize_signatures",
        deserialize_with = "deserialize_signatures"
    )]
    pub all_full_signatures: Vec<(Signature, RecoveryId)>,
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

pub fn serialize_signatures<S>(
    signatures: &Vec<(Signature, RecoveryId)>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    use serde::ser::SerializeSeq;

    let mut seq = serializer.serialize_seq(Some(signatures.len()))?;
    for (signature, recovery_id) in signatures {
        let mut bytes = signature.to_bytes().to_vec();
        bytes.push(recovery_id.to_byte());
        seq.serialize_element(&bytes)?;
    }
    seq.end()
}

// Custom deserialization function
pub fn deserialize_signatures<'de, D>(
    deserializer: D,
) -> Result<Vec<(Signature, RecoveryId)>, D::Error>
where
    D: Deserializer<'de>,
{
    struct SignatureVisitor;

    impl<'de> Visitor<'de> for SignatureVisitor {
        type Value = Vec<(Signature, RecoveryId)>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a sequence of signature byte arrays")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut signatures = Vec::new();

            while let Some(bytes) = seq.next_element::<Vec<u8>>()? {
                if bytes.len() != 65 {
                    return Err(de::Error::custom("Invalid signature length"));
                }

                let signature = Signature::try_from(&bytes[..64]).map_err(de::Error::custom)?;

                let recovery_id = RecoveryId::try_from(bytes[64]).map_err(de::Error::custom)?;

                signatures.push((signature, recovery_id));
            }

            Ok(signatures)
        }
    }

    deserializer.deserialize_seq(SignatureVisitor)
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
