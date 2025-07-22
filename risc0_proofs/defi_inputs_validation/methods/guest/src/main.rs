use ethereum_types::{Address, U256};
use merkle_verifier_core::merkle_patricia::{
    verify_account_proof, verify_all_account_proofs, verify_all_storage_proofs, AccountMerkleProof,
    StorageMerkleProof,
};
use nullifier_verifier_core::nullifiers::verify_all_nullifiers;
use risc0_zkvm::guest::env;
use shared::{DefiProofInput, DefiProofOutput};
use signature_verifier_core::signature_verifier::verify_all_signatures;

fn main() {
    let DefiProofInput {
        all_signatures,
        all_nullifiers,
        owned_accounts_addresses,
        owned_accounts_merkle_proofs,
        storage_merkle_proofs,
        contract_merkle_proof,
        contract_address,
        user_address,
        message,
    } = env::read();

    let start = env::cycle_count();
    let mut last = start;
    let mut now = start;

    // 1. Verifying all owned account proofs and get the total eth balance of all owned accounts
    let total_wei_balance: U256 =
        verify_all_account_proofs(&owned_accounts_merkle_proofs, &owned_accounts_addresses);
    let eth_divisor = U256::exp10(18); // 10^18
    let total_eth_balance = total_wei_balance / eth_divisor;

    now = env::cycle_count();
    eprintln!("{}: verify_all_account_proofs ", now - last);
    last = now;

    // 2. Verifying merkle proof of lending contract to ensure that defi data came from our
    //    contract
    //  2.1 Verify merkle proof
    let contract = verify_account_proof(
        contract_merkle_proof.state_root,
        &contract_merkle_proof.address,
        &contract_merkle_proof.account_proof,
    )
    .unwrap()
    .unwrap();
    now = env::cycle_count();
    eprintln!("{}: verify contract merkle proof", now - last);
    last = now;

    //2.2 Chcking if contract address is correct
    assert!(contract_merkle_proof
        .address
        .eq(contract_address.as_bytes()));

    // 3. Verifying merklpe proofs of all storge slots of the user and retriving their values
    let user_history_data = verify_all_storage_proofs(
        &storage_merkle_proofs,
        &contract.storage_root,
        &user_address,
    );
    assert!(user_history_data.len() == 4, "User Histor data mismatch");

    now = env::cycle_count();
    eprintln!("{}: verify_all_storage_proofs", now - last);
    last = now;

    // 4. verify the signatures
    let signatures_valid =
        verify_all_signatures(&message, &all_signatures, &owned_accounts_addresses);
    assert!(signatures_valid, "Singatures Unvalid");

    now = env::cycle_count();
    eprintln!("{}: verify_all_signatures", now - last);
    last = now;

    // 5. verify the nullifers
    let nullifiers_valid =
        verify_all_nullifiers(&all_nullifiers, &all_signatures, &owned_accounts_addresses);
    assert!(nullifiers_valid, "Nullifiers Unvalid");

    now = env::cycle_count();
    eprintln!("{}: verify_all_nullifiers", now - last);
    last = now;

    eprintln!("{}: Total Cycyles", now - start);

    let output: DefiProofOutput = DefiProofOutput {
        all_nullifiers,
        contract_address,
        user_address,
        message,
        total_eth_balance,
        first_interaction_timestamp: user_history_data[0],
        liquidations: user_history_data[1],
        on_time_payments: user_history_data[2],
        current_debt: user_history_data[3],
    };

    // write public output to the journal
    env::commit(&output);
}
