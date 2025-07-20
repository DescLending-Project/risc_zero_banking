// For RISC Zero guest usage, uncomment the following:
// use risc0_zkvm::guest::env;

use ethereum_types::Address;
use k256::SecretKey;
use k256::ecdsa::{RecoveryId, Signature, SigningKey, VerifyingKey};
use sha3::digest::consts::{B0, B1};
use sha3::digest::generic_array::GenericArray;
use sha3::digest::typenum::{UInt, UTerm};
use sha3::{Digest, Keccak256};

pub fn sign_ethereum_message(
    message: &[u8],
    private_key: &[u8; 32],
) -> Result<(Signature, RecoveryId), &'static str> {
    // Create signing key from private key
    let secret_key = SecretKey::from_slice(private_key).map_err(|_| "Invalid private key")?;
    let signing_key = SigningKey::from(secret_key);

    // Create Ethereum message hash
    let prefix = format!("\x19Ethereum Signed Message:\n{}", message.len());
    let mut full_message = prefix.into_bytes();
    full_message.extend_from_slice(message);

    let message_hash = Keccak256::digest(&full_message);

    // Sign with recovery
    let full_signature = signing_key
        .sign_prehash_recoverable(&message_hash)
        .map_err(|_| "Failed to sign message")?;

    // Combine signature and recovery ID
    // let mut sig_bytes = [0u8; 65];
    // sig_bytes[..64].copy_from_slice(&signature.to_bytes());
    // sig_bytes[64] = recovery_id.to_byte();
    // let sig_bytes = signature.0.to_bytes();

    Ok(full_signature)
}

pub fn verify_ethereum_msg_signature(
    message: &str,
    signature: Signature,
    recovery_id: RecoveryId,
    expected_address: Address,
) -> Result<bool, &'static str> {
    let message_bytes = message.as_bytes();
    // Create Ethereum message hash
    let prefix = format!("\x19Ethereum Signed Message:\n{}", message_bytes.len());
    let mut full_message = prefix.into_bytes();
    full_message.extend_from_slice(message_bytes);
    let message_hash = Keccak256::digest(&full_message);

    return verify_ethereum_signature(&message_hash, &signature, recovery_id, &expected_address);
}

pub fn verify_ethereum_signature(
    message_hash: &GenericArray<
        u8,
        UInt<UInt<UInt<UInt<UInt<UInt<UTerm, B1>, B0>, B0>, B0>, B0>, B0>,
    >,
    signature: &Signature,
    recovery_id: RecoveryId,
    expected_address: &Address,
) -> Result<bool, &'static str> {
    // Create Ethereum message hash
    // let prefix = format!("\x19Ethereum Signed Message:\n{}", message.len());
    // let mut full_message = prefix.into_bytes();
    // full_message.extend_from_slice(message);
    //
    // let message_hash = Keccak256::digest(&full_message);

    // Recover public key
    let recovered_key = VerifyingKey::recover_from_prehash(message_hash, signature, recovery_id)
        .map_err(|_| "Failed to recover public key")?;

    // Convert to Ethereum address
    let public_key_bytes = recovered_key.to_encoded_point(false);
    let pub_key_hash = Keccak256::digest(&public_key_bytes.as_bytes()[1..]);
    let derived_address = Address::from_slice(&pub_key_hash[12..]);

    Ok(expected_address.eq(&derived_address))
}

pub fn verify_all_signatures(
    message: &str,
    all_signatures: &Vec<(Signature, RecoveryId)>,
    all_addresses: &Vec<Address>,
) -> bool {
    let message_bytes = message.as_bytes();
    // Create Ethereum message hash
    let prefix = format!("\x19Ethereum Signed Message:\n{}", message_bytes.len());
    let mut full_message = prefix.into_bytes();
    full_message.extend_from_slice(message_bytes);
    let message_hash = Keccak256::digest(&full_message);
    for (index, signature) in all_signatures.iter().enumerate() {
        let is_valid = verify_ethereum_signature(
            &message_hash,
            &signature.0,
            signature.1,
            &all_addresses[index],
        )
        .unwrap();
        if !is_valid {
            return false;
        }
    }

    return true;
}

pub fn generate_all_signatures(
    user_private_keys: Vec<[u8; 32]>,
    message: &str,
) -> Vec<(Signature, RecoveryId)> {
    let mut all_signatures: Vec<(Signature, RecoveryId)> = vec![];

    for private_key in user_private_keys {
        let sig = sign_ethereum_message(&message.as_bytes(), &private_key).unwrap();
        all_signatures.push(sig);
    }

    return all_signatures;
}
#[cfg(test)]
mod tests {
    use super::*;
    use hex;

    #[test]
    fn test_anvil_signature_signing_and_verification() {
        // Default anvil account #0
        // Private key: 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
        // Address: 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266
        let anvil_private_key =
            hex::decode("ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
                .unwrap();
        let anvil_address =
            Address::from_slice(&hex::decode("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266").unwrap());

        // Message to sign
        let message = "Hello, RISC Zero!";
        let prefix = format!("\x19Ethereum Signed Message:\n{}", message.len());
        let mut full_message = prefix.into_bytes();
        full_message.extend_from_slice(message.as_bytes());

        let message_hash = Keccak256::digest(&full_message);

        // Sign the message
        let full_signature =
            sign_ethereum_message(message.as_bytes(), &anvil_private_key.try_into().unwrap())
                .unwrap();

        // Verify the signature
        let is_valid = verify_ethereum_signature(
            &message_hash,
            &full_signature.0,
            full_signature.1,
            &anvil_address,
        )
        .unwrap();

        assert!(is_valid, "Signature verification should pass");

        // Test with different message (should fail)
        let different_message = "Different message";
        let is_valid_different = verify_ethereum_msg_signature(
            different_message,
            full_signature.0,
            full_signature.1,
            anvil_address,
        )
        .unwrap();

        assert!(
            !is_valid_different,
            "Signature verification should fail for different message"
        );

        // Test with different address (should fail)
        let different_address =
            Address::from_slice(&hex::decode("70997970C51812dc3A010C7d01b50e0d17dc79C8").unwrap());
        let is_valid_different_address = verify_ethereum_msg_signature(
            message,
            full_signature.0,
            full_signature.1,
            different_address,
        )
        .unwrap();

        assert!(
            !is_valid_different_address,
            "Signature verification should fail for different address"
        );

        println!("✅ All tests passed!");
        println!("Message: {}", String::from_utf8_lossy(message.as_bytes()));
        println!("Signature: {:?}", full_signature);
        println!("Address: {}", anvil_address);
    }

    #[test]
    fn test_generate_all_signatures() {
        // Default anvil accounts (first 5)
        let anvil_private_keys = vec![
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

        let message = "Multi-signature test message";

        // Generate all signatures
        let full_signatures = generate_all_signatures(anvil_private_keys.clone(), message);

        // Verify we got the right number of signatures
        assert_eq!(full_signatures.len(), 5, "Should generate 5 signatures");

        // Verify each signature individually
        let anvil_addresses = vec![
            Address::from_slice(&hex::decode("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266").unwrap()), // Account 0
            Address::from_slice(&hex::decode("70997970C51812dc3A010C7d01b50e0d17dc79C8").unwrap()), // Account 1
            Address::from_slice(&hex::decode("3C44CdDdB6a900fa2b585dd299e03d12FA4293BC").unwrap()), // Account 2
            Address::from_slice(&hex::decode("90F79bf6EB2c4f870365E785982E1f101E93b906").unwrap()), // Account 3
            Address::from_slice(&hex::decode("15d34AAf54267DB7D7c367839AAf71A00a2C6A65").unwrap()), // Account 4
        ];

        for (i, full_signature) in full_signatures.iter().enumerate() {
            let is_valid = verify_ethereum_msg_signature(
                message,
                full_signature.0,
                full_signature.1,
                anvil_addresses[i],
            )
            .unwrap();
            assert!(is_valid, "Signature {} should be valid", i);
        }

        println!("✅ Generate all signatures test passed!");
    }

    #[test]
    fn test_verify_all_signatures() {
        // Default anvil accounts (first 3 for this test)
        let anvil_private_keys = vec![
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
        ];

        let anvil_addresses = vec![
            Address::from_slice(&hex::decode("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266").unwrap()), // Account 0
            Address::from_slice(&hex::decode("70997970C51812dc3A010C7d01b50e0d17dc79C8").unwrap()), // Account 1
            Address::from_slice(&hex::decode("3C44CdDdB6a900fa2b585dd299e03d12FA4293BC").unwrap()), // Account 2
        ];

        let message = "Batch verification test";

        // Generate signatures for all accounts
        let signatures = generate_all_signatures(anvil_private_keys.clone(), message);

        // Test successful verification
        let all_valid = verify_all_signatures(&message, &signatures, &anvil_addresses.clone());
        assert!(all_valid, "All signatures should be valid");

        // Test with wrong message (should fail)
        let wrong_message = "Wrong message";
        let all_valid_wrong =
            verify_all_signatures(&wrong_message, &signatures, &anvil_addresses.clone());
        assert!(
            !all_valid_wrong,
            "Verification should fail with wrong message"
        );

        // Test with wrong address (should fail)
        let mut wrong_addresses = anvil_addresses.clone();
        wrong_addresses[1] =
            Address::from_slice(&hex::decode("9965507D1a55bcC2695C58ba16FB37d819B0A4dc").unwrap()); // Different address
        let all_valid_wrong_addr = verify_all_signatures(&message, &signatures, &wrong_addresses);
        assert!(
            !all_valid_wrong_addr,
            "Verification should fail with wrong address"
        );

        // Test with mismatched signature (should fail)
        let mut wrong_signatures = signatures.clone();
        wrong_signatures[0] = signatures[1]; // Use signature from account 1 for account 0
        let all_valid_wrong_sig =
            verify_all_signatures(message, &wrong_signatures, &anvil_addresses.clone());
        assert!(
            !all_valid_wrong_sig,
            "Verification should fail with mismatched signature"
        );

        println!("✅ Verify all signatures test passed!");
    }
}
