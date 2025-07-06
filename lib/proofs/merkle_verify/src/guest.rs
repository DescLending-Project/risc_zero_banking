use ethereum_types::U256;
use fetch_merkle::UserHistoryProof;
use merkle_verifier_core::merkle_patricia::{
    AccountData, AccountMerkleProof, verify_account_proof, verify_all_account_proofs,
    verify_all_storage_proofs, verify_storage_proof,
};
pub struct AllMerkleProofs {
    pub user_history_proof: UserHistoryProof,
    pub owned_accounts_merkle_proofs: Vec<AccountMerkleProof>,
}

pub async fn verify_all_merkle_proofs(all_merkle_proofs: AllMerkleProofs) {
    // 1. Verifying all owned account proofs and getting the total eth balance of all owned accounts
    let mut total_eth_balance: U256 =
        verify_all_account_proofs(&all_merkle_proofs.owned_accounts_merkle_proofs);

    // 2. Verifying merkle proof of lending contract to ensure that defi data came from our
    //    contract
    let contract_merkle_proof = all_merkle_proofs.user_history_proof.contract_merkle_proof;
    let contract = verify_account_proof(
        contract_merkle_proof.state_root,
        &contract_merkle_proof.address,
        &contract_merkle_proof.account_proof,
    );

    // 3. Verifying merklpe proofs of all storge slots of the user and retriving their values
    let user_history_data =
        verify_all_storage_proofs(&all_merkle_proofs.user_history_proof.storage_merkle_proofs);

    println!("\nTotal eth balance: {:?}", total_eth_balance);
    println!("Constract: {:?}", contract);
    println!("User history: {:?}", user_history_data);
}
pub async fn verify_and_convert_defi_data(all_merkle_proofs: AllMerkleProofs) {
    verify_all_merkle_proofs(all_merkle_proofs);
    // TODO:
    // 1. Verify the signatures created with the owned user accounts
    // 2. Verify the integrity of the nullifiers hash(pubAddressN) == nullifierN
}
