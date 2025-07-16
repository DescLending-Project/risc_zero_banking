use ethereum_types::Address;
use sha2::{Digest as Sha2Digest, Sha256};

pub fn hash_address_and_signature_sha256(address: &Address, signature: &[u8; 65]) -> [u8; 32] {
    let mut hasher = Sha256::new();

    // Add address bytes
    hasher.update(address.as_bytes());

    // Add signature bytes
    hasher.update(signature);

    // Return hash as 32-byte array
    hasher.finalize().into()
}
pub fn generate_nullifier(user_address: &Address, signature: &[u8; 65]) -> [u8; 32] {
    return hash_address_and_signature_sha256(user_address, signature);
}
pub fn generate_all_nullifiers(
    all_user_signatures: &Vec<[u8; 65]>,
    all_user_addresses: &Vec<Address>,
) -> Vec<[u8; 32]> {
    let mut all_nullifiers: Vec<[u8; 32]> = vec![];

    for (index, signature) in all_user_signatures.iter().enumerate() {
        let nullifier = generate_nullifier(&all_user_addresses[index], &signature);
        all_nullifiers.push(nullifier);
    }

    return all_nullifiers;
}

pub fn verify_nullifier(
    nullifier: &[u8; 32],
    user_address: &Address,
    signature: &[u8; 65],
) -> bool {
    return generate_nullifier(user_address, signature).eq(nullifier);
}

pub fn verify_all_nullifiers(
    all_nullifiers: &Vec<[u8; 32]>,
    all_signatures: &Vec<[u8; 65]>,
    all_addresses: &Vec<Address>,
) -> bool {
    for (index, signature) in all_signatures.iter().enumerate() {
        let is_valid = verify_nullifier(&all_nullifiers[index], &all_addresses[index], &signature);
        if !is_valid {
            return false;
        }
    }

    return true;
}
