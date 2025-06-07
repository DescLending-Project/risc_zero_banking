// Copyright 2024 RISC Zero, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// This application demonstrates how to send an off-chain proof request
// to the Bonsai proving service and publish the received proofs directly
// to your deployed app contract.

// use alloy::{
//     network::EthereumWallet, providers::ProviderBuilder, signers::local::PrivateKeySigner,
//     sol_types::SolValue,
// };
use clap::Parser;
use methods::MERKEL_ELF;
use risc0_ethereum_contracts::encode_seal;
use url::Url;

use anyhow::{Context, Result};
use bincode;
use risc0_groth16::docker::stark_to_snark;
use risc0_zkvm::Prover;
use risc0_zkvm::{
    default_executor, default_prover, serde::from_slice, ExecutorEnv, ProverOpts, Receipt,
    ReceiptKind, VerifierContext,
};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::env;
use std::fs;
use std::time::Instant;
// `IEvenNumber` interface automatically generated via the alloy `sol!` macro.
// alloy::sol!(
//     #[sol(rpc, all_derives)]
//     "../contracts/IEvenNumber.sol"
// );

/// Arguments of the publisher CLI.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Ethereum chain ID
    #[clap(long)]
    chain_id: u64,

    /// Ethereum Node endpoint.
    // #[clap(long, env)]
    // eth_wallet_private_key: PrivateKeySigner,

    /// Ethereum Node endpoint.
    // #[clap(long)]
    // rpc_url: Url,
    //
    // /// Application's contract address on Ethereum
    // #[clap(long)]
    // contract: Address,
    //
    // /// The input to provide to the guest binary
    #[clap(short, long)]
    input: U256,
}

#[derive(Deserialize, Serialize)]
struct ProofPrivateInput {
    state_root: [u8; 32],
    address: [u8; 20],
    slot_key: Option<[u8; 32]>,
    account_proof: Vec<Vec<u8>>,
    storage_proof: Option<Vec<Vec<u8>>>,
}

#[derive(Deserialize, Serialize)]
struct ProofPublicInput {
    address: [u8; 20],
}

#[derive(Deserialize)]
struct ProofOutput {
    address: Option<[u8; 20]>,
    balance: Option<U256>,
}
#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    // Parse CLI Arguments: The application starts by parsing command-line arguments provided by the user.
    let args = Args::parse();
    //
    // Try multiple provider options
    let provider = setup_eth_provider().await?;

    // Address we want to verify
    let address = "0x8d0BB74e37ab644964AcA2f3Fbe12b9147f9d841".parse::<Address>()?;

    // Optional storage slot
    let slot = Some(H256::zero()); // Example: first storage slot

    // Get a state root hash at a specific block
    let block_number = 22406754;
    let block = BlockId::Number(BlockNumber::Number(block_number.into()));

    println!("Getting block details for block: {}", block_number);
    let block_data = provider
        .get_block(block)
        .await?
        .context("Block not found")?;

    let state_root = block_data.state_root;
    println!("Got state root: 0x{}", hex::encode(state_root.as_bytes()));

    println!(
        "Getting proof for address: {} at block: {}",
        address, block_number
    );

    // Fetch the proof from the Ethereum node using eth_getProof
    let mut proof = provider
        .get_proof(address, vec![slot.unwrap_or_default()], Some(block))
        .await?;

    println!("Got proof, account has balance: {}", proof.balance);

    // Convert proof to the format expected by our verifier
    let account_proof: Vec<Vec<u8>> = proof
        .account_proof
        .into_iter()
        .map(|bytes| bytes.to_vec())
        .collect();

    // Only include storage proof if we specified a slot
    let storage_proof = if slot.is_some() {
        Some(
            proof.storage_proof[0]
                .proof
                .clone()
                .into_iter()
                .map(|bytes| bytes.to_vec())
                .collect(),
        )
    } else {
        None
    };

    // Prepare input for RISC Zero guest
    let public_input = ProofPublicInput {
        address: address.into(),
    };
    let private_input = ProofPrivateInput {
        state_root: state_root.into(),
        address: address.into(),
        slot_key: slot.map(|h| h.into()),
        account_proof,
        storage_proof,
    };

    // Create an alloy provider for that private key and URL.
    // let wallet = EthereumWallet::from(args.eth_wallet_private_key);
    // let provider = ProviderBuilder::new()
    //     .wallet(wallet)
    //     .connect_http(args.rpc_url);

    // ABI encode input: Before sending the proof request to the Bonsai proving service,
    // the input number is ABI-encoded to match the format expected by the guest code running in the zkVM.
    // let input = args.input.abi_encode();

    // let prove_env = ExecutorEnv::builder().write(&public_input)?.build()?;
    let opts = ProverOpts::default().with_receipt_kind(ReceiptKind::Succinct);
    // let opts = ProverOpts::groth16();
    let env = ExecutorEnv::builder()
        .write(&public_input)
        .unwrap()
        .write(&private_input)?
        .build()
        .unwrap();
    let start = Instant::now();
    println!("Started proving:");
    let receipt = default_prover()
        .prove_with_ctx(env, &VerifierContext::default(), MERKEL_ELF, &opts)?
        .receipt;

    let duration = start.elapsed();
    println!("Time taken: {:?}", duration);
    // Encode the seal with the selector.
    // let seal = encode_seal(&receipt)?;
    // println!("{:?}", seal);

    // Extract the journal from the receipt.
    let journal = receipt.journal.bytes.clone();

    // Decode Journal: Upon receiving the proof, the application decodes the journal to extract
    // the verified number. This ensures that the number being submitted to the blockchain matches
    // the number that was verified off-chain.
    let output: ProofOutput = from_slice(&receipt.journal.bytes)?;

    // Print the results
    println!("ZK proof verification successful!");
    // println!("Account exists: {}", output.balance != None);

    match output.balance {
        Some(balance) => {
            println!("Account details from ZK proof:");
            println!("  Balance: {}", balance);
        }
        None => {
            println!("No balance returned");
        }
    }

    // Construct function call: Using the IEvenNumber interface, the application constructs
    // the ABI-encoded function call for the set function of the EvenNumber contract.
    // This call includes the verified number, the post-state digest, and the seal (proof).

    // let contract = IEvenNumber::new(args.contract, provider);
    // let call_builder = contract.set(x, seal.into());

    // Initialize the async runtime environment to handle the transaction sending.
    // let runtime = tokio::runtime::Runtime::new()?;

    // Send transaction: Finally, send the transaction to the Ethereum blockchain,
    // effectively calling the set function of the EvenNumber contract with the verified number and proof.

    // let pending_tx = runtime.block_on(call_builder.send())?;
    // runtime.block_on(pending_tx.get_receipt())?;

    Ok(())
}

// Function to try multiple provider options
async fn setup_eth_provider() -> Result<Provider<Http>> {
    // Try Alchemy if environment variable exists
    if let Ok(alchemy_key) = env::var("ALCHEMY_API_KEY") {
        let url = format!("https://eth-mainnet.alchemyapi.io/v2/{}", alchemy_key);
        let provider = Provider::<Http>::try_from(url.as_str())?;

        // Test the connection
        if let Ok(_) = provider.get_block_number().await {
            println!("Connected to Ethereum via Alchemy");
            return Ok(provider);
        }
    }
    // If all else fails, try Ethereum archive RPCs from eth-archive.r2
    let url = "https://eth-archive.r2.scorched.io/";
    println!("Trying fallback archive node: {}", url);
    let provider = Provider::<Http>::try_from(url)?;

    // Test the connection
    provider.get_block_number().await?;
    println!("Connected to Ethereum via archive node");

    Ok(provider)
}
