use ethereum_types::Address;
use k256::ecdsa::{RecoveryId, Signature, SigningKey, VerifyingKey};
use sha2::{Digest as Sha2Digest, Sha256};
pub fn hash_address_and_signature_sha256(address: &Address, signature: &Signature) -> [u8; 32] {
    let mut hasher = Sha256::new();

    // Add address bytes
    hasher.update(address.as_bytes());

    // Add signature bytes
    hasher.update(signature.to_bytes());

    // Return hash as 32-byte array
    hasher.finalize().into()
}

pub fn generate_nullifier(user_address: &Address, signature: &Signature) -> [u8; 32] {
    return hash_address_and_signature_sha256(user_address, signature);
}
pub fn generate_all_nullifiers(
    all_full_signatures: &Vec<(Signature, RecoveryId)>,
    all_user_addresses: &Vec<Address>,
) -> Vec<[u8; 32]> {
    let mut all_nullifiers: Vec<[u8; 32]> = vec![];

    for (index, full_signature) in all_full_signatures.iter().enumerate() {
        let nullifier = generate_nullifier(&all_user_addresses[index], &full_signature.0);
        all_nullifiers.push(nullifier);
    }

    return all_nullifiers;
}

pub fn verify_nullifier(
    nullifier: &[u8; 32],
    user_address: &Address,
    signature: &Signature,
) -> bool {
    return generate_nullifier(user_address, signature).eq(nullifier);
}

pub fn verify_all_nullifiers(
    all_nullifiers: &Vec<[u8; 32]>,
    all_full_signatures: &Vec<(Signature, RecoveryId)>,
    all_addresses: &Vec<Address>,
) -> bool {
    for (index, signature) in all_full_signatures.iter().enumerate() {
        let is_valid =
            verify_nullifier(&all_nullifiers[index], &all_addresses[index], &signature.0);
        if !is_valid {
            return false;
        }
    }

    return true;
}
