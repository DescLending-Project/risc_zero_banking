#[cfg(test)]
mod tests {
    use ethers::{
        core::types::{Address, EIP1186ProofResponse, H256, U256},
        providers::{Http, Middleware, Provider},
        utils::keccak256,
    };
    use merkle_verifier_core::fetch_merkle_proofs::*;
    use std::{str::FromStr, vec};
    #[tokio::test]
    async fn verify_user_history_proof() {}
}
