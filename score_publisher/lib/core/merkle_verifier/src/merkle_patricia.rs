extern crate alloc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use ethereum_types::{Address, H256, U256};
use rlp::{DecoderError, Rlp};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};

// Custom error type that can convert from both DecoderError and &str
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProofError {
    RlpDecoding(String),
    InvalidProof(String),
    HashMismatch(String),
    InvalidPath(String),
}

// Implement conversion from DecoderError to ProofError
impl From<DecoderError> for ProofError {
    fn from(err: DecoderError) -> Self {
        ProofError::RlpDecoding(err.to_string())
    }
}

// Implement conversion from &str to ProofError
impl From<&str> for ProofError {
    fn from(msg: &str) -> Self {
        ProofError::InvalidProof(msg.to_string())
    }
}
// Account data structure matching Ethereum's state trie format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountData {
    pub nonce: U256,
    pub balance: U256,
    pub storage_root: H256,
    pub code_hash: H256,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct AccountMerkleProof {
    pub state_root: H256,
    pub address: [u8; 20],
    pub account_proof: Vec<Vec<u8>>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct StorageMerkleProof {
    pub state_root: H256,
    pub key: [u8; 32],
    pub storage_proof: Vec<Vec<u8>>,
}

// Define the basic node types as before
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Node {
    Empty,
    Leaf {
        key: Vec<u8>,
        value: Vec<u8>,
    },
    Extension {
        key: Vec<u8>,
        node: H256,
    },
    Branch {
        children: [Option<H256>; 16],
        value: Option<Vec<u8>>,
    },
}

// Basic nibble utilities for working with hex prefixed keys
pub struct NibbleSlice<'a>(&'a [u8]);

impl<'a> NibbleSlice<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        NibbleSlice(data)
    }

    pub fn at(&self, i: usize) -> u8 {
        if i % 2 == 0 {
            self.0[i / 2] >> 4
        } else {
            self.0[i / 2] & 0x0f
        }
    }

    pub fn len(&self) -> usize {
        self.0.len() * 2
    }
}

// Basic proof verification function
pub fn verify_proof(
    root_hash: H256,
    key: &[u8],
    proof: &[Vec<u8>],
) -> Result<Option<Vec<u8>>, ProofError> {
    // Convert key to nibbles for trie traversal
    let key_nibbles = encode_path(key);

    // Start with the root hash as our expected hash
    let mut expected_hash = root_hash;

    // Current position in the path
    let mut key_index = 0;

    // Process each node in the proof
    for node_data in proof {
        // Calculate actual hash of this node for verification
        let actual_hash = H256::from_slice(&Keccak256::digest(node_data).as_slice());

        // Verify this node matches what we expect
        if actual_hash != expected_hash {
            // println!("Acutuall thing");
            // println!("{:?}", actual_hash);
            // println!("Expected thing");
            // println!("{:?}", expected_hash);

            return Err(ProofError::HashMismatch(
                "Invalid proof: hash mismatch".into(),
            ));
        }

        // Parse the RLP-encoded node
        let rlp = Rlp::new(node_data);

        // Branch node (has 17 items: 16 children + value)
        if rlp.item_count()? == 17 {
            // We've consumed the entire path - return the value at this branch
            if key_index >= key_nibbles.len() {
                let value_item = rlp.at(16)?;
                if value_item.is_empty() {
                    return Ok(None); // No value at this node
                } else {
                    return Ok(Some(value_item.as_val()?));
                }
            }

            // Otherwise, get the next nibble from our key and follow that branch
            let nibble = key_nibbles[key_index] as usize;
            key_index += 1;

            let child = rlp.at(nibble)?;
            if child.is_empty() {
                return Ok(None); // No child here means no value exists
            }

            // Extract the next hash to follow
            expected_hash = child.as_val()?;
        }
        // Leaf or extension node (has 2 items)
        else if rlp.item_count()? == 2 {
            let path_item = rlp.at(0)?;
            let path: Vec<u8> = path_item.as_val()?;

            // Decode the compact encoding to get the node type and actual path
            let (node_type, node_path) = decode_compact(path.as_slice());

            match node_type {
                // Leaf node - terminal node with a value
                0x2 | 0x3 => {
                    // Check remaining key matches node path
                    let remaining_key = &key_nibbles[key_index..];
                    if remaining_key != node_path.as_slice() {
                        return Ok(None); // Path doesn't match our key
                    }

                    // Return the value
                    let value_item = rlp.at(1)?;
                    return Ok(Some(value_item.as_val()?));
                }

                // Extension node - internal node that compresses shared path
                0x0 | 0x1 => {
                    // Check that the path matches our key segment
                    let path_len = node_path.len();
                    if key_index + path_len > key_nibbles.len() {
                        return Ok(None); // Key too short
                    }

                    let key_segment = &key_nibbles[key_index..key_index + path_len];
                    if key_segment != node_path.as_slice() {
                        return Ok(None); // Path doesn't match
                    }

                    // Update key index to skip the matched segment
                    key_index += path_len;

                    // Get the next node to look up
                    let value_item = rlp.at(1)?;
                    expected_hash = value_item.as_val()?;
                }

                _ => return Err(ProofError::InvalidPath("Invalid node type".into())),
            }
        } else {
            return Err(ProofError::InvalidProof("Invalid node format".into()));
        }
    }

    Err(ProofError::InvalidProof("Proof too short".into()))
}

// Convert normal bytes to nibbles
pub fn encode_path(key: &[u8]) -> Vec<u8> {
    let mut nibbles = Vec::with_capacity(key.len() * 2);
    for &byte in key {
        nibbles.push(byte >> 4);
        nibbles.push(byte & 0x0f);
    }
    nibbles
}

// Decode compact encoding used in Ethereum's tries
pub fn decode_compact(encoded: &[u8]) -> (u8, Vec<u8>) {
    let mut result = Vec::new();
    let mut node_type = encoded[0] >> 4;

    // First nibble contains the type flag
    if node_type == 0 || node_type == 2 {
        // Even number of nibbles
        for &byte in &encoded[1..] {
            result.push(byte >> 4);
            result.push(byte & 0x0f);
        }
    } else {
        // Odd number of nibbles
        result.push(encoded[0] & 0x0f);
        for &byte in &encoded[1..] {
            result.push(byte >> 4);
            result.push(byte & 0x0f);
        }
    }

    (node_type, result)
}

// pub fn verify_account_proof2(
//     proof: &AccountMerkleProof,
// ) -> Result<Option<AccountData>, ProofError> {
//     // We need to hash the address with keccak256 to get the key for the proof.state trie
//     let address_hash = Keccak256::digest(proof.address).to_vec();
//
//     // Verify the proof to get the account RLP
//     let account_rlp = match verify_proof(proof.state_root, &address_hash, &proof.account_proof)? {
//         Some(data) => data,
//         None => return Ok(None), // Account doesn't exist
//     };
//
//     // Decode the account data
//     let rlp = Rlp::new(&account_rlp);
//
//     let account = AccountData {
//         nonce: rlp.at(0)?.as_val()?,
//         balance: rlp.at(1)?.as_val()?,
//         storage_root: rlp.at(2)?.as_val()?,
//         code_hash: rlp.at(3)?.as_val()?,
//     };
//
//     Ok(Some(account))
// }

pub fn verify_account_proof(
    state_root: H256,
    address: &[u8; 20],
    account_proof: &[Vec<u8>],
) -> Result<Option<AccountData>, ProofError> {
    // We need to hash the address with keccak256 to get the key for the state trie
    let address_hash = Keccak256::digest(address).to_vec();

    // Verify the proof to get the account RLP
    let account_rlp = match verify_proof(state_root, &address_hash, account_proof)? {
        Some(data) => data,
        None => return Ok(None), // Account doesn't exist
    };

    // Decode the account data
    let rlp = Rlp::new(&account_rlp);

    let account = AccountData {
        nonce: rlp.at(0)?.as_val()?,
        balance: rlp.at(1)?.as_val()?,
        storage_root: rlp.at(2)?.as_val()?,
        code_hash: rlp.at(3)?.as_val()?,
    };

    Ok(Some(account))
}

pub fn verify_storage_proof(
    storage_root: H256,
    key: &[u8; 32],
    storage_proof: &[Vec<u8>],
) -> Result<Option<U256>, ProofError> {
    // Hash the key with keccak256
    let key_hash = Keccak256::digest(key).to_vec();

    // Verify the proof
    match verify_proof(storage_root, &key_hash, storage_proof)? {
        Some(value_rlp) => {
            let rlp = Rlp::new(&value_rlp);
            let value: U256 = rlp.as_val()?;
            Ok(Some(value))
        }
        None => Ok(None),
    }
}

/// Calculate storage slot for mapping(address => UserHistory) users
pub fn calculate_mapping_slot(user_address: &Address, mapping_slot: U256) -> H256 {
    // For mapping(address => UserHistory), storage slot = keccak256(abi.encodePacked(key, slot))
    let mut data = Vec::new();

    // Add user address (32 bytes, left-padded)
    let mut addr_bytes = [0u8; 32];
    addr_bytes[12..].copy_from_slice(user_address.as_bytes());
    data.extend_from_slice(&addr_bytes);

    // Add mapping slot (32 bytes)
    let mut slot_bytes = [0u8; 32];
    mapping_slot.to_big_endian(&mut slot_bytes);
    data.extend_from_slice(&slot_bytes);

    let hash: [u8; 32] = Keccak256::digest(&data).into();
    // let xd = Keccak256::digest(&data);
    // H256(Keccak256::digest(&data).into())
    H256::from(hash)
}

pub fn calculate_struct_slots(base_slot: H256) -> [H256; 4] {
    let base_u256 = U256::from_big_endian(base_slot.as_bytes());

    let slot1 = base_u256 + U256::one();
    let slot2 = base_u256 + U256::from(2);
    let slot3 = base_u256 + U256::from(3);

    let mut slot1_bytes = [0u8; 32];
    let mut slot2_bytes = [0u8; 32];
    let mut slot3_bytes = [0u8; 32];
    slot1.to_big_endian(&mut slot1_bytes);
    slot2.to_big_endian(&mut slot2_bytes);
    slot3.to_big_endian(&mut slot3_bytes);

    [
        base_slot,               // firstInteractionTimestamp
        H256::from(slot1_bytes), // liquidations
        H256::from(slot2_bytes), // successfulPayments
        H256::from(slot3_bytes), // total_dept
    ]
}
pub fn verify_all_storage_proofs(
    storage_proofs: &Vec<StorageMerkleProof>,
    state_root: &H256,
    user_address: &Address,
) -> Vec<U256> {
    let mapping_slot = U256::zero(); //NOTE: here it is assumed that  users mapping is at slot 0 of the contratc. TODO: THis should be dynamicly set!
    let user_base_slot = calculate_mapping_slot(user_address, mapping_slot);
    let user_hisotry_slots = calculate_struct_slots(user_base_slot);
    let mut values: Vec<U256> = vec![];
    // println!("{:?}", storage_proofs);
    for (index, proof) in storage_proofs.iter().enumerate() {
        print!("Index {:?} val: ", index);
        // Checking if the state_root is correct
        assert!(
            proof.state_root.eq(state_root),
            "State roots mismatch expected {:?} got {:?}",
            state_root,
            proof.state_root
        );
        // Check if the field is the field owned by the user
        assert!(proof.key.eq(user_hisotry_slots[index].as_bytes()));

        let value = match verify_storage_proof(proof.state_root, &proof.key, &proof.storage_proof) {
            Ok(Some(val)) => val,
            Ok(None) => panic!("Storage proof verification failed - no return data"),
            Err(e) => panic!(
                "Storage proof verification error : {:?} \n for the proof {:?}",
                e, proof
            ),
        };
        values.push(value);
    }
    return values;
}

pub fn verify_all_account_proofs(
    owned_accounts_merkle_proofs: &Vec<AccountMerkleProof>,
    owned_accounts_addresses: &Vec<Address>,
    state_root: &H256,
) -> U256 {
    let mut total_eth_balance: U256 = U256::from(0);
    for (index, proof) in owned_accounts_merkle_proofs.iter().enumerate() {
        print!("{:?}", index);

        assert!(
            proof.state_root.eq(state_root),
            "State roots mismatch expected {:?} got {:?}",
            state_root,
            proof.state_root
        );
        let account: AccountData =
            match verify_account_proof(proof.state_root, &proof.address, &proof.account_proof) {
                Ok(Some(account_data)) => account_data,
                Ok(None) => panic!("Account proof verification failed - no account data"),
                Err(e) => panic!(
                    "Account proof verification error : {:?} \n for the proof {:?}",
                    e, proof
                ),
            };
        assert!(
            proof
                .address
                .eq(&owned_accounts_addresses[index].as_bytes()),
            "Account address and AccountMerkleProof address mismatch"
        );

        total_eth_balance += account.balance;
        // println!("{:?}", account);
    }
    return total_eth_balance;
}
