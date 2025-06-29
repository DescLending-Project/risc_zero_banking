use anyhow::{Context, Result};
use ethers::types::BlockNumber;
use ethers::utils::hash_message;
use ethers::{
    providers::{Http, Middleware, Provider},
    signers::{LocalWallet, Signer},
    types::{Address, BlockId, EIP1186ProofResponse, H256, Signature},
    utils::hex,
};
use std::env;
use std::str::FromStr;

use fetch_merkle::{MerkleProofFetcher, UserHistoryProof};

use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct AnvilAccount {
    pub address: String,
    #[serde(rename = "privateKey")]
    pub private_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnvilAccountsData {
    pub accounts: Vec<AnvilAccount>,
}

pub enum Node {
    Anvil,
    Alchemy,
    Eth_Archive,
}
// Function to sign a message with any private key
pub async fn sign_message(
    private_key: &str,
    message: &str,
) -> Result<Signature, Box<dyn std::error::Error>> {
    let wallet = LocalWallet::from_str(private_key)?;
    let signature = wallet.sign_message(message).await?;

    println!("Signed message: '{}'", message);
    println!("With wallet: {:?}", wallet.address());
    println!("Signature: {:?}", signature);

    Ok(signature)
}
// Function to sign multiple messages with one private key
pub async fn batch_sign_message(
    message: &str,
    private_keys: &[&str],
) -> Result<Vec<Signature>, Box<dyn std::error::Error>> {
    let mut signatures = Vec::new();
    for (i, private_key) in private_keys.iter().enumerate() {
        let wallet = LocalWallet::from_str(private_key)?;
        println!("\nBatch signing with wallet: {:?}", wallet.address());
        let signature = wallet.sign_message(message).await?;
        let is_valid = verify_signature(message, &signature, wallet.address())?;
        println!("Message {}: '{}' -> Valid: {}", i + 1, message, is_valid);
        signatures.push(signature);
    }

    Ok(signatures)
}

// Function to sign multiple messages with one private key
pub async fn batch_sign_messages(
    private_key: &str,
    messages: &[&str],
) -> Result<Vec<Signature>, Box<dyn std::error::Error>> {
    let wallet = LocalWallet::from_str(private_key)?;
    let mut signatures = Vec::new();

    println!("\nBatch signing with wallet: {:?}", wallet.address());

    for (i, message) in messages.iter().enumerate() {
        let signature = wallet.sign_message(message).await?;
        let is_valid = verify_signature(message, &signature, wallet.address())?;
        println!("Message {}: '{}' -> Valid: {}", i + 1, message, is_valid);
        signatures.push(signature);
    }

    Ok(signatures)
}

pub fn verify_signature(
    message: &str,
    signature: &Signature,
    expected_address: Address,
) -> Result<bool, Box<dyn std::error::Error>> {
    // Hash the message using Ethereum's message hashing (adds prefix)
    let message_hash = hash_message(message);

    // Recover the address from signature
    let recovered_address = signature.recover(message_hash)?;

    // Check if recovered address matches expected address
    Ok(recovered_address == expected_address)
}

// Function to verify multiple signatures at once
pub fn verify_signatures(
    messages: &[&str],
    signatures: &[Signature],
    expected_address: Address,
) -> Result<bool, Box<dyn std::error::Error>> {
    if messages.len() != signatures.len() {
        return Err("Messages and signatures length mismatch".into());
    }

    for (message, signature) in messages.iter().zip(signatures.iter()) {
        let is_valid = verify_signature(message, signature, expected_address)?;
        if (!is_valid) {
            return Ok(false);
        }
    }

    Ok(true)
}
// Function to verify multiple signatures at once
pub fn batch_verify_signatures(
    messages: &[&str],
    signatures: &[Signature],
    expected_address: Address,
) -> Result<Vec<bool>, Box<dyn std::error::Error>> {
    if messages.len() != signatures.len() {
        return Err("Messages and signatures length mismatch".into());
    }

    let mut results = Vec::new();
    for (message, signature) in messages.iter().zip(signatures.iter()) {
        let is_valid = verify_signature(message, signature, expected_address)?;
        results.push(is_valid);
    }

    Ok(results)
}

// Function to get wallet address from private key
pub fn get_wallet_address(private_key: &str) -> Result<Address, Box<dyn std::error::Error>> {
    let wallet = LocalWallet::from_str(private_key)?;
    Ok(wallet.address())
}

// TODO
// 1.write functions that sin msg and verify them
// 2. saef and load the anvvil account_address
// 3. in the loop
// a) load eth_merkle_proof
// b) sign message
// 4. store al of this as 2 seperat jsons
// 5. write loop that takes this json, verifys the balances and signatures and returns total owned
//    balance
//    we only need to verify the statemerkel hash once !!!!
//      all rest of the prooving does not need the tlsn
//      merkel proof will assume that risc0 proof was c
//

// Function to try multiple provider options
// For easier testing and development: in production frontend will generate the tlsn representation
// that we will have to parce
pub async fn setup_eth_provider(node_type: Node) -> Result<Provider<Http>> {
    // Try connecting to locally runing anvil node
    //
    match node_type {
        Node::Anvil => {
            let provider = Provider::<Http>::try_from("http://localhost:8545")?;
            // Test the connection
            if let Ok(_) = provider.get_block_number().await {
                println!("Connected to local Anvil node");
                return Ok(provider);
            }
        }
        Node::Alchemy => {
            if let Ok(alchemy_key) = env::var("ALCHEMY_API_KEY") {
                let url = format!("https://eth-mainnet.alchemyapi.io/v2/{}", alchemy_key);
                let provider = Provider::<Http>::try_from(url.as_str())?;

                // Test the connection
                if let Ok(_) = provider.get_block_number().await {
                    println!("Connected to Ethereum via Alchemy");
                    return Ok(provider);
                }
            }
        }
        _ => {
            println!("Gonna try archive node")
        }
    }
    let url = "https://eth-archive.r2.scorched.io/";
    println!("Trying fallback archive node: {}", url);
    let provider = Provider::<Http>::try_from(url)?;

    // Test the connection
    provider.get_block_number().await?;
    println!("Connected to Ethereum via archive node");

    return Ok(provider);
    // if ALCHEMY_API_KEY is set then try alchemyapi
}
pub async fn get_user_history_merkle_proof(
    provider: Provider<Http>,
    user_address: Address,
    contract_address: Address,
) -> Result<EIP1186ProofResponse> {
    let fetcher = MerkleProofFetcher::new("http://localhost:8545", Some(provider)).unwrap();
    let complete_user_data = fetcher
        .fetch_complete_user_data(contract_address, user_address)
        .await
        .unwrap();

    return Ok(complete_user_data.merkle_proof);
}
pub async fn get_user_complete_history(
    provider: Provider<Http>,
    user_address: Address,
    contract_address: Address,
) -> Result<UserHistoryProof> {
    let fetcher = MerkleProofFetcher::new("http://localhost:8545", Some(provider)).unwrap();
    let complete_user_data = fetcher
        .fetch_complete_user_data(contract_address, user_address)
        .await
        .unwrap();

    return Ok(complete_user_data);
}

// pub async fn get_account_merkle_proof(
//     provider: Provider<Http>,
//     account_address: Address,
//     block: BlockId,
// ) -> Result<EIP1186ProofResponse> {
//     // Address we want to verify
//
//     // Optional storage slot
//     let slot = Some(H256::zero()); // Example: first storage slot
//
//     println!("Getting block details for block: {:?}", block);
//     let block_data = provider
//         .get_block(block)
//         .await?
//         .context("Block not found")?;
//
//     let state_root = block_data.state_root;
//     println!("Got state root: 0x{}", hex::encode(state_root.as_bytes()));
//
//     println!(
//         "Getting proof for address: {} at block: {:?}",
//         account_address, block
//     );
//
//     // Fetch the proof from the Ethereum node using eth_getProof
//     let mut proof = provider
//         .get_proof(account_address, vec![slot.unwrap_or_default()], None)
//         .await?;
//
//     println!("Got proof, account has balance: {}", proof.balance);
//
//     return Ok(proof);
// }
