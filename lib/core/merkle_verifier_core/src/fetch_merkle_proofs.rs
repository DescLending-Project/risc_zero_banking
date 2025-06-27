use ethers::{
    core::types::{Address, EIP1186ProofResponse, H256, U256},
    providers::{Http, Middleware, Provider},
    utils::keccak256,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{str::FromStr, vec};
use tokio;

#[derive(Debug, Serialize, Deserialize)]
pub struct UserHistory {
    pub first_interaction_timestamp: U256,
    pub liquidations: U256,
    pub successful_payments: U256,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StorageSlot {
    pub slot: H256,
    pub value: H256,
    pub decoded_value: U256,
    pub field_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserHistoryProof {
    pub contract_address: Address,
    pub user_address: Address,
    pub block_number: String,
    pub user_history: UserHistory,
    pub storage_slots: Vec<StorageSlot>,
    pub merkle_proof: EIP1186ProofResponse,
    pub metadata: ProofMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProofMetadata {
    pub fetched_at: String,
    pub rpc_url: String,
    pub mapping_slot: U256,
    pub base_storage_slot: H256,
}

pub struct MerkleProofFetcher {
    pub provider: Provider<Http>,
    pub rpc_url: String,
}

impl MerkleProofFetcher {
    pub fn new(
        rpc_url: &str,
        provider: Option<Provider<Http>>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        match provider {
            Some(provider) => Ok(Self {
                provider,
                rpc_url: rpc_url.to_string(),
            }),
            None => {
                let provider = Provider::<Http>::try_from(rpc_url)?;
                Ok(Self {
                    provider,
                    rpc_url: rpc_url.to_string(),
                })
            }
        }
    }

    /// Calculate storage slot for mapping(address => UserHistory) users
    pub fn calculate_mapping_slot(&self, user_address: Address, mapping_slot: U256) -> H256 {
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

        H256(keccak256(&data))
    }

    /// Calculate storage slots for UserHistory struct fields
    pub fn calculate_struct_slots(&self, base_slot: H256) -> [H256; 3] {
        let base_u256 = U256::from_big_endian(base_slot.as_bytes());

        let slot1 = base_u256 + U256::one();
        let slot2 = base_u256 + U256::from(2);

        let mut slot1_bytes = [0u8; 32];
        let mut slot2_bytes = [0u8; 32];
        slot1.to_big_endian(&mut slot1_bytes);
        slot2.to_big_endian(&mut slot2_bytes);

        [
            base_slot,               // firstInteractionTimestamp
            H256::from(slot1_bytes), // liquidations
            H256::from(slot2_bytes), // successfulPayments
        ]
    }

    /// Fetch storage values for the user history
    pub async fn fetch_user_history(
        &self,
        contract_address: Address,
        user_address: Address,
    ) -> Result<UserHistory, Box<dyn std::error::Error>> {
        let mapping_slot = U256::zero(); //NOTE: here it is assumed that  users mapping is at slot 0 of the contratc. TODO: THis should be dynamicly set!
        let base_slot = self.calculate_mapping_slot(user_address, mapping_slot);
        println!("base slot:");
        println!("{:?}", base_slot);
        let struct_slots = self.calculate_struct_slots(base_slot);

        let timestamp = self
            .provider
            .get_storage_at(contract_address, struct_slots[0], None)
            .await?;
        let liquidations = self
            .provider
            .get_storage_at(contract_address, struct_slots[1], None)
            .await?;
        let payments = self
            .provider
            .get_storage_at(contract_address, struct_slots[2], None)
            .await?;

        Ok(UserHistory {
            first_interaction_timestamp: U256::from_big_endian(timestamp.as_bytes()),
            liquidations: U256::from_big_endian(liquidations.as_bytes()),
            successful_payments: U256::from_big_endian(payments.as_bytes()),
        })
    }

    /// Fetch merkle proofs using eth_getProof
    pub async fn fetch_merkle_proof(
        &self,
        contract_address: Address,
        user_address: Address,
    ) -> Result<EIP1186ProofResponse, Box<dyn std::error::Error>> {
        let mapping_slot = U256::zero();
        let base_slot = self.calculate_mapping_slot(user_address, mapping_slot);
        let struct_slots = self.calculate_struct_slots(base_slot);

        // Convert storage slots to the format expected by eth_getProof
        let storage_keys: Vec<H256> = struct_slots.to_vec();

        // let response: Value = self.provider.request("eth_getProof", Some(params)).await?;
        let response: EIP1186ProofResponse = self
            .provider
            .get_proof(contract_address, storage_keys, None)
            .await?;

        println!("{}", serde_json::to_string_pretty(&response).unwrap());
        // Parse the respons
        // let account_proof: AccountProof = serde_json::from_value(response)?;
        // println!("accoutn proof");
        // println!("{:?}", account_proof);
        Ok(response)
    }

    /// Main function to fetch complete user history with proofs
    pub async fn fetch_complete_user_data(
        &self,
        contract_address: Address,
        user_address: Address,
    ) -> Result<UserHistoryProof, Box<dyn std::error::Error>> {
        println!(
            "🔍 Fetching data for user {} from contract {}",
            user_address, contract_address
        );

        // Calculate storage layout
        let mapping_slot = U256::zero();
        let base_slot = self.calculate_mapping_slot(user_address, mapping_slot);
        let struct_slots = self.calculate_struct_slots(base_slot);

        // Fetch user history values
        let user_history = self
            .fetch_user_history(contract_address, user_address)
            .await?;

        // Fetch merkle proofs
        let merkle_proof = self
            .fetch_merkle_proof(contract_address, user_address)
            .await?;

        // Get current block number
        let block_number = self.provider.get_block_number().await?;

        // Create storage slot information
        let field_names = [
            "firstInteractionTimestamp",
            "liquidations",
            "successfulPayments",
        ];
        let values = [
            user_history.first_interaction_timestamp,
            user_history.liquidations,
            user_history.successful_payments,
        ];

        let storage_slots: Vec<StorageSlot> = struct_slots
            .iter()
            .zip(field_names.iter())
            .zip(values.iter())
            .map(|((slot, name), value)| {
                let mut value_bytes = [0u8; 32];
                value.to_big_endian(&mut value_bytes);
                StorageSlot {
                    slot: *slot,
                    value: H256::from(value_bytes),
                    decoded_value: *value,
                    field_name: name.to_string(),
                }
            })
            .collect();

        // Create metadata
        let metadata = ProofMetadata {
            fetched_at: chrono::Utc::now().to_rfc3339(),
            rpc_url: self.rpc_url.clone(),
            mapping_slot,
            base_storage_slot: base_slot,
        };

        Ok(UserHistoryProof {
            contract_address,
            user_address,
            block_number: block_number.to_string(),
            user_history,
            storage_slots,
            merkle_proof,
            metadata,
        })
    }

    /// Save the complete data to a JSON file
    pub async fn save_to_file(
        &self,
        data: &UserHistoryProof,
        filename: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let json_data = serde_json::to_string_pretty(data)?;
        tokio::fs::write(filename, json_data).await?;
        println!("💾 Data saved to {}", filename);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configuration
    let rpc_url = "http://localhost:8545"; // Anvil default
                                           // lending contract address 0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512
    let contract_address = Address::from_str("0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512")?;
    // placeholder lending contract will create history for 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266
    let user_address = Address::from_str("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266")?; // Default Anvil account

    // Create fetcher
    let fetcher = MerkleProofFetcher::new(rpc_url, None)?;

    // Fetch complete data
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
                "user_history_proof_{}_{}.json",
                contract_address
                    .to_string()
                    .replace("0x", "")
                    .chars()
                    .take(8)
                    .collect::<String>(),
                user_address
                    .to_string()
                    .replace("0x", "")
                    .chars()
                    .take(8)
                    .collect::<String>()
            );

            fetcher.save_to_file(&data, &filename).await?;

            // Also print formatted JSON to console
            println!("\n📄 Complete JSON Data:");
            println!("{}", serde_json::to_string_pretty(&data)?);
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

    Ok(())
}
