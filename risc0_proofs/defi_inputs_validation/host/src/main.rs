use clap::Parser;
#[cfg(fetch_merkle_proofs)]
use ethereum_types::Address;
use ethers::abi::AbiEncode;
use ethers::types::{BlockId, BlockNumber};
use futures::executor::block_on;
use loaders::loaders::{
    load_all_merkle_proofs, load_nullifiers, load_signatures, load_user_owned_addresses,
};
use methods::{DEFI_INPUTS_VALIDATOR_ELF, DEFI_INPUTS_VALIDATOR_ID};
use risc0_zkvm::{default_prover, ExecutorEnv, ProverOpts};
use shared::{DefiProofInput, DefiProofOutput};
use std::io::Cursor;
use std::{env::args, fs, time::Instant};
use tokio::runtime::Runtime;

#[cfg(fetch_merkle_proofs)]
use fetch_merkle::MerkleProofFetcher;

#[derive(Parser, Debug)]
struct Args {
    #[clap(long)]
    all_signatures_path: String,

    #[clap(long)]
    all_nullifiers_path: String,

    #[clap(long)]
    user_owned_addresses_path: String,

    #[cfg(not(fetch_merkle_proofs))]
    #[clap(long)]
    all_merkle_proofs_path: String,

    #[clap(long)]
    proof_name: String,

    #[clap(long)]
    bin_output_path: String,

    #[cfg(fetch_merkle_proofs)]
    #[clap(long)]
    api_url: String,

    #[cfg(fetch_merkle_proofs)]
    #[clap(long)]
    contract_address: Address,
}

fn main() {
    // Initialize tracing. In order to view logs, run `RUST_LOG=info cargo run`
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    // 1. Loading the defi data that is supposed to be verified:
    // NOTE: in this case the signatres and nullifiers where generated with message = "Block 2"
    let message = "Block 2";
    let all_signatures = load_signatures(args.all_signatures_path).unwrap();
    let all_nullifiers = load_nullifiers(args.all_nullifiers_path).unwrap();
    let user_owned_addresses = load_user_owned_addresses(args.user_owned_addresses_path).unwrap();

    // #[cfg(not(fetch_merkle_proofs))]
    // let all_merkle_proofs = load_all_merkle_proofs(args.all_merkle_proofs_path).unwrap();
    // // in production the host of defi inputs validation needs to fetch all of the merkle proofs
    //
    #[cfg(not(fetch_merkle_proofs))]
    let all_merkle_proofs = load_all_merkle_proofs(args.all_merkle_proofs_path).unwrap();

    #[cfg(fetch_merkle_proofs)]
    let all_merkle_proofs = {
        let fetcher = MerkleProofFetcher::new(&args.api_url, None).unwrap();
        let block_id = BlockId::Number(BlockNumber::Latest);
        let rt = Runtime::new().unwrap();
        rt.block_on(fetcher.fetch_all_merkle_proofs(
            args.contract_address.clone(),
            user_owned_addresses[0].clone(),
            user_owned_addresses.clone(),
            block_id,
        ))
    }
    .unwrap();
    let proofInputs: DefiProofInput = DefiProofInput {
        all_signatures,
        all_nullifiers,
        owned_accounts_addresses: user_owned_addresses.clone(),
        owned_accounts_merkle_proofs: all_merkle_proofs.owned_accounts_merkle_proofs,
        storage_merkle_proofs: all_merkle_proofs.user_history_proof.storage_merkle_proofs,
        contract_merkle_proof: all_merkle_proofs.user_history_proof.contract_merkle_proof,
        contract_address: all_merkle_proofs.user_history_proof.contract_address,
        user_address: all_merkle_proofs.user_history_proof.user_address,
        message: message.to_string(),
        trusted_state_root: all_merkle_proofs.user_history_proof.state_root,
    };
    // Create a buffer to capture stderr
    let stderr_buffer = Vec::new();
    // passing inputs to the guest
    let env = ExecutorEnv::builder()
        .write(&proofInputs)
        .unwrap()
        .stderr(std::io::stderr())
        .build()
        .unwrap();

    // Obtain the default prover.
    let prover = default_prover();

    // Proof information by proving the specified ELF binary.
    // This struct contains the receipt along with statistics about execution of the guest
    // let prove_info = prover.prove(env, DEFI_INPUTS_VALIDATOR_ELF).unwrap();
    let start = Instant::now();
    println!("Running the prover...");
    let opts = ProverOpts::succinct();
    let prove_info = prover
        .prove_with_opts(env, DEFI_INPUTS_VALIDATOR_ELF, &opts)
        .unwrap();

    // extract the receipt.
    let receipt = prove_info.receipt;

    // The receipt was verified at the end of proving, but the below code is an
    // example of how someone else could verify this receipt.
    receipt.verify(DEFI_INPUTS_VALIDATOR_ID).unwrap();
    println!("Receipt verification successful!");

    // decoding journal
    let _output: DefiProofOutput = receipt.journal.decode().unwrap();
    println!("Output: {:?}", _output);

    // Save the receipt to receipt.bin
    println!("\nSaving receipt to receipt.bin...");
    let receipt_bytes = bincode::serialize(&receipt).unwrap();
    let bytes_len = receipt_bytes.len();
    fs::create_dir_all(&args.bin_output_path).unwrap();

    let out = args.bin_output_path + &args.proof_name + ".bin";
    fs::write(out.clone(), receipt_bytes).unwrap();

    println!("Receipt saved to {:?} ({} bytes)", out, bytes_len);
    let duration = start.elapsed();
    println!("{} :Time elapsed in ms", duration.as_millis());
    println!("{} :Accounts count", user_owned_addresses.len());

    let stderr_output = String::from_utf8(stderr_buffer).unwrap();
    println!("Guest stderr output: {}", stderr_output);
}
