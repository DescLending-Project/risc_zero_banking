# Defi inputs validation proof
This proof has 2 main task: 1. Verify that the borrower is the owner of provided accounts for score calculation 2. Verify and ensure the integrity of blockchain orginated score calculation inputs  
Dependednt on RUSTFLAGS="--cfg fetch_merkle_proofs" the host program will fetch all needed merkle proofs from specified node provider address(api_url) or it will use data specified by --all-merkle-proofs-path
The guest program gets following inputs:
```rust
pub struct DefiProofInput {
    pub all_signatures: Vec<[u8; 65]>,
    pub all_nullifiers: Vec<[u8; 32]>,
    pub owned_accounts_addresses: Vec<Address>,
    pub owned_accounts_merkle_proofs: Vec<AccountMerkleProof>,
    pub storage_merkle_proofs: Vec<StorageMerkleProof>,
    pub contract_merkle_proof: AccountMerkleProof,
    pub contract_address: Address,
    pub user_address: Address,
    pub message: String,
    pub trusted_state_root: H256,
}

```
We first verify the all_signatures and their relation to owned_accounts_addresses to fulfill task 1.
We also verify the relation between the accounts nullifiers(all_nullifers) and the owned_accounts_addresses.
Next the guest program checks the integrity of blockchain  orginated data with merkle_proofs verification by:
1. Verifying the existence and  integrity of owned_accounts and their ballancess.
2. Verifying  the existence and integrity of the lending contract 
3. Verifying  the existence and integrity of user interactions data coming from lending contract


Guest program stores following struct that is stored afterwards in receipt binarry that can be used afterwards by the nesting proof:
```rust
pub struct DefiProofOutput {
    #[serde(
        serialize_with = "serialize_nullifiers",
        deserialize_with = "deserialize_nullifiers"
    )]
    pub all_nullifiers: Vec<[u8; 32]>,
    pub contract_address: Address,
    pub user_address: Address,
    pub message: String,
    pub total_eth_balance: U256,
    // Defi Payment history
    pub first_interaction_timestamp: U256,
    pub liquidations: U256,
    pub on_time_payments: U256,
    pub trusted_state_root: H256,
}
```
NOTE: 
1. trusted_state_root will be checked in nesting proof against state_root fetched with via tlsn, ensuring its integrity. The nesting proof also outputs the orgin url of the state_root that gets checked on chain against an list of accepted node providers. The node provider URL (at least in case of ALCHEMY) determinse the source Blockchain of the data. 
2. contract_address is the address of the lending contract and it gets checked on chain 



## Quick Start

First, make sure [rustup] is installed. The
[`rust-toolchain.toml`][rust-toolchain] file will be used by `cargo` to
automatically install the correct version.

To build all methods and execute the method within the zkVM in DEV mode, run the following
command:

```bash
RISC0_DEV_MODE=1 cargo run  -- \
--all-signatures-path ./defi_inputs/signatures.json \
--all-nullifiers-path ./defi_inputs/nullifiers.json \
--all-merkle-proofs-path ./defi_inputs/all_merkle_proofs.json \
--user-owned-addresses-path ./defi_inputs/user_owned_addresses.json \
--proof-name unvalid_defi_inputs_receipt \
--bin-output-path ./receipts/5_accounts_proof/ \
```

Depenedent on the RUSTFLAGS="--cfg fetch_merkle_proofs" you can execute the proof with locally stored merkle proofs(upper command) or you can specify the node provider url and the address of the lending contract.
The receipt will be stored in --bin-output-path under --proof-name.



```bash
RISC0_DEV_MODE=1 RUSTFLAGS="--cfg fetch_merkle_proofs" cargo run  -- \
--all-signatures-path ./defi_inputs/signatures.json \
--all-nullifiers-path ./defi_inputs/nullifiers.json \
--user-owned-addresses-path ./defi_inputs/user_owned_addresses.json \
--proof-name unvalid_defi_inputs_receipt \
--bin-output-path ./receipts/5_accounts_proof/ \
--api-url http://localhost:8545 \
--contract-address 0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512
```

You you want to generate valid receipt change the RISC0_DEV_MODE to 0



### Running Proofs Remotely on Bonsai

_Note: The Bonsai proving service is still in early Alpha; an API key is
required for access. [Click here to request access][bonsai access]._

If you have access to the URL and API key to Bonsai you can run your proofs
remotely. You need to export the bonsai api key and url first in the terminal:

```bash
export BONSAI_API_KEY="<BONSAI_API_KEY>"
export BONSAI_API_URL="<BONSAI_API_URL>"
```

Then run the proof in normal mode
```bash
RISC0_DEV_MODE=0 cargo run --release -- \
--all-signatures-path ./defi_inputs/signatures.json \
--all-nullifiers-path ./defi_inputs/nullifiers.json \
--all-merkle-proofs-path ./defi_inputs/all_merkle_proofs.json \
--user-owned-addresses-path ./defi_inputs/user_owned_addresses.json \
--proof-name valid_defi_inputs_receipt \
--bin-output-path ./receipts/5_accounts_proof/ \
```



## Questions, Feedback, and Collaborations

We'd love to hear from you on [Discord][discord] or [Twitter][twitter].

[bonsai access]: https://bonsai.xyz/apply
[cargo-risczero]: https://docs.rs/cargo-risczero
[crates]: https://github.com/risc0/risc0/blob/main/README.md#rust-binaries
[dev-docs]: https://dev.risczero.com
[dev-mode]: https://dev.risczero.com/api/generating-proofs/dev-mode
[discord]: https://discord.gg/risczero
[docs.rs]: https://docs.rs/releases/search?query=risc0
[examples]: https://github.com/risc0/risc0/tree/main/examples
[risc0-build]: https://docs.rs/risc0-build
[risc0-repo]: https://www.github.com/risc0/risc0
[risc0-zkvm]: https://docs.rs/risc0-zkvm
[rust-toolchain]: rust-toolchain.toml
[rustup]: https://rustup.rs
[twitter]: https://twitter.com/risczero
[zkhack-iii]: https://www.youtube.com/watch?v=Yg_BGqj_6lg&list=PLcPzhUaCxlCgig7ofeARMPwQ8vbuD6hC5&index=5
[zkvm-overview]: https://dev.risczero.com/zkvm
