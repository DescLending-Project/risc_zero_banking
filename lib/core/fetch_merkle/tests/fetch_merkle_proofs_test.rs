#[cfg(test)]
mod tests {
    use ethers::{
        core::types::{Address, EIP1186ProofResponse, H256, U256},
        providers::{Http, Middleware, Provider},
        utils::keccak256,
    };
    use std::str::FromStr;

    use fetch_merkle::*;
    #[tokio::test]
    async fn fetch_and_store_data() {
        let fetcher = MerkleProofFetcher::new("http://localhost:8545", None).unwrap();
        let contract_address =
            Address::from_str("0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512").unwrap();
        let user_address = Address::from_str("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266").unwrap();
        let mapping_slot = U256::zero();

        let base_slot = fetcher.calculate_mapping_slot(user_address, mapping_slot);
        let struct_slots = fetcher.calculate_struct_slots(base_slot);

        println!("Base slot: {:?}", base_slot);
        println!("Struct slots: {:?}", struct_slots);

        match fetcher
            .fetch_complete_user_data(contract_address, user_address)
            .await
        {
            Ok(data) => {
                // Print summary
                println!("\n📊 User History Summary:");
                println!(
                    "  First Interaction: {}",
                    data.user_history.first_interaction_timestamp
                );
                println!("  Liquidations: {}", data.user_history.liquidations);
                println!(
                    "  Successful Payments: {}",
                    data.user_history.successful_payments
                );
                println!("  Base Storage Slot: {:?}", data.metadata.base_storage_slot);

                // Print storage slots
                println!("\n🗄️  Storage Slots:");
                for slot in &data.storage_slots {
                    println!(
                        "  {}: {} (slot: {:?})",
                        slot.field_name, slot.decoded_value, slot.slot
                    );
                }

                // Save to file
                let filename = format!(
                    "user_history_proof_{}.json",
                    contract_address
                        .to_string()
                        .replace("0x", "")
                        .chars()
                        .take(8)
                        .collect::<String>()
                );

                fetcher.save_to_file(&data, &filename).await.unwrap();

                // Also print formatted JSON to console
                println!("\n📄 Complete JSON Data:");
                println!("{}", serde_json::to_string_pretty(&data).unwrap());
            }
            Err(e) => {
                eprintln!("❌ Error fetching data: {}", e);

                // Try basic connectivity test
                println!("\n🔧 Testing basic connectivity...");
                match fetcher.provider.get_block_number().await {
                    Ok(block) => println!("✅ Connected to RPC. Current block: {}", block),
                    Err(e) => println!("❌ RPC connection failed: {}", e),
                }
            }
        }
    }

    #[tokio::test]
    async fn test_storage_slot_calculation() {
        let fetcher = MerkleProofFetcher::new("http://localhost:8545", None).unwrap();
        let user_address = Address::from_str("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266").unwrap();
        let mapping_slot = U256::zero();

        let base_slot = fetcher.calculate_mapping_slot(user_address, mapping_slot);
        let struct_slots = fetcher.calculate_struct_slots(base_slot);

        println!("Base slot: {:?}", base_slot);
        println!("Struct slots: {:?}", struct_slots);

        // Verify that struct slots are consecutive
        let base_u256 = U256::from_big_endian(base_slot.as_bytes());
        let expected_slot1 = base_u256 + U256::one();
        let expected_slot2 = base_u256 + U256::from(2);

        let mut slot1_bytes = [0u8; 32];
        let mut slot2_bytes = [0u8; 32];
        expected_slot1.to_big_endian(&mut slot1_bytes);
        expected_slot2.to_big_endian(&mut slot2_bytes);

        assert_eq!(struct_slots[1], H256::from(slot1_bytes));
        assert_eq!(struct_slots[2], H256::from(slot2_bytes));
    }

    #[tokio::test]
    async fn test_user_history_fetch() {
        let fetcher = MerkleProofFetcher::new("http://localhost:8545", None).unwrap();
        let contract_address =
            Address::from_str("0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512").unwrap();
        // placeholder lending contract will create history for 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266
        let user_address = Address::from_str("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266").unwrap(); // Default Anvil account

        // This test requires Anvil to be running with the contract deployed
        match fetcher
            .fetch_user_history(contract_address, user_address)
            .await
        {
            Ok(history) => {
                println!("✅ Successfully fetched user history: {:?}", history);
            }
            Err(e) => {
                println!(
                    "⚠️  Could not fetch user history (Anvil not running?): {}",
                    e
                );
            }
        }
    }
}
