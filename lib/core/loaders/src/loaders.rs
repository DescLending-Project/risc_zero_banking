use ethereum_types::Address;
use fetch_merkle::AllMerkleProofs;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json;
use std::fs;
use std::path::Path;

// Wrapper type for [u8; 32] that implements Serialize/Deserialize
#[derive(Debug, Clone, PartialEq)]
struct Array32([u8; 32]);

impl Serialize for Array32 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for Array32 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec: Vec<u8> = Vec::deserialize(deserializer)?;
        if vec.len() != 32 {
            return Err(serde::de::Error::custom(format!(
                "Expected array of length 32, got {}",
                vec.len()
            )));
        }
        let mut array = [0u8; 32];
        array.copy_from_slice(&vec);
        Ok(Array32(array))
    }
}

// Wrapper type for [u8; 65] that implements Serialize/Deserialize
#[derive(Debug, Clone, PartialEq)]
struct Array65([u8; 65]);

impl Serialize for Array65 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for Array65 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec: Vec<u8> = Vec::deserialize(deserializer)?;
        if vec.len() != 65 {
            return Err(serde::de::Error::custom(format!(
                "Expected array of length 65, got {}",
                vec.len()
            )));
        }
        let mut array = [0u8; 65];
        array.copy_from_slice(&vec);
        Ok(Array65(array))
    }
}

/// Save AllMerkleProofs to a JSON file
pub fn save_all_merkle_proofs<P: AsRef<Path>>(
    proofs: &AllMerkleProofs,
    file_path: P,
) -> Result<(), Box<dyn std::error::Error>> {
    let json_string = serde_json::to_string_pretty(proofs)?;
    fs::write(file_path, json_string)?;
    Ok(())
}

/// Load AllMerkleProofs from a JSON file
pub fn load_all_merkle_proofs<P: AsRef<Path>>(
    file_path: P,
) -> Result<AllMerkleProofs, Box<dyn std::error::Error>> {
    let json_string = fs::read_to_string(file_path)?;
    let proofs: AllMerkleProofs = serde_json::from_str(&json_string)?;
    Ok(proofs)
}

/// Save signatures to a JSON file
pub fn save_signatures<P: AsRef<Path>>(
    all_signatures: &Vec<[u8; 65]>,
    file_path: P,
) -> Result<(), Box<dyn std::error::Error>> {
    // Convert to wrapper type for serialization
    let wrapped_signatures: Vec<Array65> = all_signatures.iter().map(|&arr| Array65(arr)).collect();
    let json_string = serde_json::to_string_pretty(&wrapped_signatures)?;
    fs::write(file_path, json_string)?;
    Ok(())
}

/// Load signatures from a JSON file
pub fn load_signatures<P: AsRef<Path>>(
    file_path: P,
) -> Result<Vec<[u8; 65]>, Box<dyn std::error::Error>> {
    let json_string = fs::read_to_string(file_path)?;
    let wrapped_signatures: Vec<Array65> = serde_json::from_str(&json_string)?;
    // Convert back to [u8; 65]
    let signatures: Vec<[u8; 65]> = wrapped_signatures
        .into_iter()
        .map(|wrapper| wrapper.0)
        .collect();
    Ok(signatures)
}

/// Save nullifiers to a JSON file
pub fn save_nullifiers<P: AsRef<Path>>(
    all_nullifiers: &Vec<[u8; 32]>,
    file_path: P,
) -> Result<(), Box<dyn std::error::Error>> {
    // Convert to wrapper type for serialization
    let wrapped_nullifiers: Vec<Array32> = all_nullifiers.iter().map(|&arr| Array32(arr)).collect();
    let json_string = serde_json::to_string_pretty(&wrapped_nullifiers)?;
    fs::write(file_path, json_string)?;
    Ok(())
}

/// Load nullifiers from a JSON file
pub fn load_nullifiers<P: AsRef<Path>>(
    file_path: P,
) -> Result<Vec<[u8; 32]>, Box<dyn std::error::Error>> {
    let json_string = fs::read_to_string(file_path)?;
    let wrapped_nullifiers: Vec<Array32> = serde_json::from_str(&json_string)?;
    // Convert back to [u8; 32]
    let nullifiers: Vec<[u8; 32]> = wrapped_nullifiers
        .into_iter()
        .map(|wrapper| wrapper.0)
        .collect();
    Ok(nullifiers)
}
