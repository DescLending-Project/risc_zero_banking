#[cfg(test)]
mod tests {
    use eth_utils::{
        batch_sign_messages, batch_verify_signatures, get_wallet_address, sign_message,
        verify_signature,
    };
    use ethers::prelude::*;
    use ethers::signers::{LocalWallet, Signer};
    use ethers::types::{Address, Signature};
    use std::str::FromStr;

    // Test constants - Anvil default accounts
    const ANVIL_PRIVATE_KEY_1: &str =
        "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
    const ANVIL_PRIVATE_KEY_2: &str =
        "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d";
    const ANVIL_ADDRESS_1: &str = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266";
    const ANVIL_ADDRESS_2: &str = "0x70997970C51812dc3A010C7d01b50e0d17dc79C8";

    #[tokio::test]
    async fn test_sign_message_basic() {
        let message = "Hello, test world!";
        let result = sign_message(ANVIL_PRIVATE_KEY_1, message).await;

        assert!(result.is_ok());
        let signature = result.unwrap();

        // Check signature has valid components
        assert_ne!(signature.r, U256::zero());
        assert_ne!(signature.s, U256::zero());
        assert!(signature.v == 27 || signature.v == 28);
    }

    #[tokio::test]
    async fn test_verify_signature_valid() {
        let message = "Test message for verification";
        let signature = sign_message(ANVIL_PRIVATE_KEY_1, message).await.unwrap();
        let address = Address::from_str(ANVIL_ADDRESS_1).unwrap();

        let is_valid = verify_signature(message, &signature, address).unwrap();
        assert!(is_valid);
    }

    #[tokio::test]
    async fn test_verify_signature_wrong_message() {
        let original_message = "Original message";
        let wrong_message = "Wrong message";

        let signature = sign_message(ANVIL_PRIVATE_KEY_1, original_message)
            .await
            .unwrap();
        let address = Address::from_str(ANVIL_ADDRESS_1).unwrap();

        let is_valid = verify_signature(wrong_message, &signature, address).unwrap();
        assert!(!is_valid);
    }

    #[tokio::test]
    async fn test_verify_signature_wrong_address() {
        let message = "Test message";
        let signature = sign_message(ANVIL_PRIVATE_KEY_1, message).await.unwrap();
        let wrong_address = Address::from_str(ANVIL_ADDRESS_2).unwrap();

        let is_valid = verify_signature(message, &signature, wrong_address).unwrap();
        assert!(!is_valid);
    }

    #[tokio::test]
    async fn test_sign_with_different_keys() {
        let message = "Same message, different signers";

        let sig1 = sign_message(ANVIL_PRIVATE_KEY_1, message).await.unwrap();
        let sig2 = sign_message(ANVIL_PRIVATE_KEY_2, message).await.unwrap();

        // Signatures should be different even for same message
        assert_ne!(sig1.r, sig2.r);
        assert_ne!(sig1.s, sig2.s);

        // But both should be valid for their respective addresses
        let addr1 = Address::from_str(ANVIL_ADDRESS_1).unwrap();
        let addr2 = Address::from_str(ANVIL_ADDRESS_2).unwrap();

        assert!(verify_signature(message, &sig1, addr1).unwrap());
        assert!(verify_signature(message, &sig2, addr2).unwrap());
    }

    #[tokio::test]
    async fn test_get_wallet_address() {
        let address = get_wallet_address(ANVIL_PRIVATE_KEY_1).unwrap();
        let expected = Address::from_str(ANVIL_ADDRESS_1).unwrap();

        assert_eq!(address, expected);
    }

    #[tokio::test]
    async fn test_get_wallet_address_invalid_key() {
        let invalid_key = "0xinvalid";
        let result = get_wallet_address(invalid_key);

        assert!(result.is_err());
    }

    // #[tokio::test]
    // async fn test_sign_and_verify() {
    //     let message = "One-shot sign and verify test";
    //     let result = sign_and_verify(ANVIL_PRIVATE_KEY_1, message).await;
    //
    //     assert!(result.is_ok());
    //     assert!(result.unwrap());
    // }

    #[tokio::test]
    async fn test_batch_sign_messages() {
        let messages = vec!["First message", "Second message", "Third message"];

        let signatures = batch_sign_messages(ANVIL_PRIVATE_KEY_1, &messages)
            .await
            .unwrap();

        assert_eq!(signatures.len(), messages.len());

        // Verify each signature
        let address = Address::from_str(ANVIL_ADDRESS_1).unwrap();
        for (message, signature) in messages.iter().zip(signatures.iter()) {
            assert!(verify_signature(message, signature, address).unwrap());
        }
    }

    #[tokio::test]
    async fn test_batch_verify_signatures() {
        let messages = vec!["Msg 1", "Msg 2", "Msg 3"];
        let signatures = batch_sign_messages(ANVIL_PRIVATE_KEY_1, &messages)
            .await
            .unwrap();
        let address = Address::from_str(ANVIL_ADDRESS_1).unwrap();

        let results = batch_verify_signatures(&messages, &signatures, address).unwrap();

        assert_eq!(results.len(), messages.len());
        assert!(results.iter().all(|&valid| valid));
    }

    #[tokio::test]
    async fn test_batch_verify_length_mismatch() {
        let messages = vec!["Msg 1", "Msg 2"];
        let signatures = batch_sign_messages(ANVIL_PRIVATE_KEY_1, &["Single msg"])
            .await
            .unwrap();
        let address = Address::from_str(ANVIL_ADDRESS_1).unwrap();

        let result = batch_verify_signatures(&messages, &signatures, address);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_empty_message() {
        let empty_message = "";
        let signature = sign_message(ANVIL_PRIVATE_KEY_1, empty_message)
            .await
            .unwrap();
        let address = Address::from_str(ANVIL_ADDRESS_1).unwrap();

        let is_valid = verify_signature(empty_message, &signature, address).unwrap();
        assert!(is_valid);
    }

    #[tokio::test]
    async fn test_unicode_message() {
        let unicode_message = "Hello, 世界! 🌍 Ethereum signatures work with unicode! 🚀";
        let signature = sign_message(ANVIL_PRIVATE_KEY_1, unicode_message)
            .await
            .unwrap();
        let address = Address::from_str(ANVIL_ADDRESS_1).unwrap();

        let is_valid = verify_signature(unicode_message, &signature, address).unwrap();
        assert!(is_valid);
    }

    #[tokio::test]
    async fn test_long_message() {
        let long_message = "A".repeat(1000);
        let signature = sign_message(ANVIL_PRIVATE_KEY_1, &long_message)
            .await
            .unwrap();
        let address = Address::from_str(ANVIL_ADDRESS_1).unwrap();

        let is_valid = verify_signature(&long_message, &signature, address).unwrap();
        assert!(is_valid);
    }

    #[tokio::test]
    async fn test_signature_deterministic() {
        let message = "Deterministic test";

        // Sign same message twice with same key
        let sig1 = sign_message(ANVIL_PRIVATE_KEY_1, message).await.unwrap();
        let sig2 = sign_message(ANVIL_PRIVATE_KEY_1, message).await.unwrap();

        // Signatures should be identical (deterministic)
        assert_eq!(sig1.r, sig2.r);
        assert_eq!(sig1.s, sig2.s);
        assert_eq!(sig1.v, sig2.v);
    }

    #[tokio::test]
    async fn test_cross_wallet_verification_fails() {
        let message = "Cross verification test";

        // Sign with wallet 1
        let signature = sign_message(ANVIL_PRIVATE_KEY_1, message).await.unwrap();

        // Try to verify with wallet 2's address (should fail)
        let wrong_address = Address::from_str(ANVIL_ADDRESS_2).unwrap();
        let is_valid = verify_signature(message, &signature, wrong_address).unwrap();

        assert!(!is_valid);
    }

    #[test]
    fn test_signature_components_range() {
        // This is more of a property test - checking signature component ranges
        // In practice, r and s should be in valid ECDSA ranges, v should be 27 or 28

        // We can't easily create invalid signatures, but we can test valid ones
        let rt = tokio::runtime::Runtime::new().unwrap();
        let signature = rt.block_on(async {
            sign_message(ANVIL_PRIVATE_KEY_1, "Range test")
                .await
                .unwrap()
        });

        // v should be 27 or 28 for Ethereum
        assert!(signature.v == 27 || signature.v == 28);

        // r and s should not be zero
        assert_ne!(signature.r, U256::zero());
        assert_ne!(signature.s, U256::zero());

        // s should be in lower half of curve order (canonical form)
        let secp256k1_order = U256::from_str_radix(
            "fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141",
            16,
        )
        .unwrap();
        let half_order = secp256k1_order / 2;
        assert!(signature.s <= half_order);
    }
}
