use eth_utils::{Node, setup_eth_provider};
use ethereum_types::Address;
use ethers::types::{BlockId, BlockNumber};
use fetch_merkle::MerkleProofFetcher;
use loaders::loaders::{
    save_all_merkle_proofs, save_nullifiers, save_signatures, save_user_owned_addresses,
};
use nullifier_verifier_core::nullifiers::generate_all_nullifiers;
use signature_verifier_core::signature_verifier::generate_all_signatures;
use std::{fs, str::FromStr};

#[tokio::main]
async fn main() {
    println!("Hello, world!");
    generate_all_evaluation_inputs().await;
}

async fn generate_defi_inputs(
    user_owned_addresses: Vec<Address>,
    user_owned_private_keys: Vec<[u8; 32]>,
    fetcher: &MerkleProofFetcher,
    block_id: BlockId,
    contract_address: Address,
) {
    let user_address = user_owned_addresses[0];
    let message = "Block 2";
    let all_signatures = generate_all_signatures(user_owned_private_keys.clone(), message);
    let all_nullifiers = generate_all_nullifiers(&all_signatures, &user_owned_addresses);
    let accounts_count = user_owned_addresses.len();
    fs::create_dir_all("./evaluation_inputs").unwrap();
    save_signatures(
        &all_signatures,
        format!("./evaluation_inputs/{}_signatures.json", accounts_count),
    )
    .unwrap();
    save_nullifiers(
        &all_nullifiers,
        format!("./evaluation_inputs/{}_nullifiers.json", accounts_count),
    )
    .unwrap();
    save_user_owned_addresses(
        &user_owned_addresses,
        format!("./evaluation_inputs/{}_accounts.json", accounts_count),
    )
    .unwrap();
    let all_merkle_proofs = fetcher
        .fetch_all_merkle_proofs(
            contract_address,
            user_address,
            user_owned_addresses.clone(),
            block_id,
        )
        .await
        .unwrap(); // Save
    save_all_merkle_proofs(
        &all_merkle_proofs,
        format!(
            "./evaluation_inputs/{}_all_merkle_proofs.json",
            accounts_count
        ),
    )
    .unwrap();
}

async fn generate_all_evaluation_inputs() {
    let user_owned_private_keys: Vec<[u8; 32]> = vec![
        hex::decode("ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
            .unwrap()
            .try_into()
            .unwrap(), // Account 0
        hex::decode("59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d")
            .unwrap()
            .try_into()
            .unwrap(), // Account 1
        hex::decode("5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a")
            .unwrap()
            .try_into()
            .unwrap(), // Account 2
        hex::decode("7c852118294e51e653712a81e05800f419141751be58f605c371e15141b007a6")
            .unwrap()
            .try_into()
            .unwrap(), // Account 3
        hex::decode("47e179ec197488593b187f80a00eb0da91f1b9d0b13f8733639f19c30a34926a")
            .unwrap()
            .try_into()
            .unwrap(), // Account 4
        hex::decode("8b3a350cf5c34c9194ca85829a2df0ec3153be0318b5e2d3348e872092edffba")
            .unwrap()
            .try_into()
            .unwrap(), // Account 5
        hex::decode("92db14e403b83dfe3df233f83dfa3a0d7096f21ca9b0d6d6b8d88b2b4ec1564e")
            .unwrap()
            .try_into()
            .unwrap(), // Account 6
        hex::decode("4bbbf85ce3377467afe5d46f804f221813b2bb87f24d81f60f1fcdbf7cbf4356")
            .unwrap()
            .try_into()
            .unwrap(), // Account 7
        hex::decode("dbda1821b80551c9d65939329250298aa3472ba22feea921c0cf5d620ea67b97")
            .unwrap()
            .try_into()
            .unwrap(), // Account 8
        hex::decode("2a871d0798f97d79848a013d4936a73bf4cc922c825d33c1cf7073dff6d409c6")
            .unwrap()
            .try_into()
            .unwrap(), // Account 9
        hex::decode("f214f2b2cd398c806f84e317254e0f0b801d0643303237d97a22a48e01628897")
            .unwrap()
            .try_into()
            .unwrap(), // Account 10
        hex::decode("701b615bbdfb9de65240bc28bd21bbc0d996645a3dd57e7b12bc2bdf6f192c82")
            .unwrap()
            .try_into()
            .unwrap(), // Account 11
        hex::decode("a267530f49f8280200edf313ee7af6b827f2a8bce2897751d06a843f644967b1")
            .unwrap()
            .try_into()
            .unwrap(), // Account 12
        hex::decode("47c99abed3324a2707c28affff1267e45918ec8c3f20b8aa892e8b065d2942dd")
            .unwrap()
            .try_into()
            .unwrap(), // Account 13
        hex::decode("c526ee95bf44d8fc405a158bb884d9d1238d99f0612e9f33d006bb0789009aaa")
            .unwrap()
            .try_into()
            .unwrap(), // Account 14
        hex::decode("8166f546bab6da521a8369cab06c5d2b9e46670292d85c875ee9ec20e84ffb61")
            .unwrap()
            .try_into()
            .unwrap(), // Account 15
        hex::decode("ea6c44ac03bff858b476bba40716402b03e41b8e97e276d1baec7c37d42484a0")
            .unwrap()
            .try_into()
            .unwrap(), // Account 16
        hex::decode("689af8efa8c651a91ad287602527f3af2fe9f6501a7ac4b061667b5a93e037fd")
            .unwrap()
            .try_into()
            .unwrap(), // Account 17
        hex::decode("de9be858da4a475276426320d5e9262ecfc3ba460bfac56360bfa6c4c28b4ee0")
            .unwrap()
            .try_into()
            .unwrap(), // Account 18
        hex::decode("df57089febbacf7ba0bc227dafbffa9fc08a93fdc68e1e42411a14efcf23656e")
            .unwrap()
            .try_into()
            .unwrap(), // Account 19
    ];

    let user_owned_addresses = vec![
        Address::from_slice(&hex::decode("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266").unwrap()), // Account 0
        Address::from_slice(&hex::decode("70997970C51812dc3A010C7d01b50e0d17dc79C8").unwrap()), // Account 1
        Address::from_slice(&hex::decode("3C44CdDdB6a900fa2b585dd299e03d12FA4293BC").unwrap()), // Account 2
        Address::from_slice(&hex::decode("90F79bf6EB2c4f870365E785982E1f101E93b906").unwrap()), // Account 3
        Address::from_slice(&hex::decode("15d34AAf54267DB7D7c367839AAf71A00a2C6A65").unwrap()), // Account 4
        Address::from_slice(&hex::decode("9965507D1a55bcC2695C58ba16FB37d819B0A4dc").unwrap()), // Account 5
        Address::from_slice(&hex::decode("976EA74026E726554dB657fA54763abd0C3a0aa9").unwrap()), // Account 6
        Address::from_slice(&hex::decode("14dC79964da2C08b23698B3D3cc7Ca32193d9955").unwrap()), // Account 7
        Address::from_slice(&hex::decode("23618e81E3f5cdF7f54C3d65f7FBc0aBf5B21E8f").unwrap()), // Account 8
        Address::from_slice(&hex::decode("a0Ee7A142d267C1f36714E4a8F75612F20a79720").unwrap()), // Account 9
        Address::from_slice(&hex::decode("Bcd4042DE499D14e55001CcbB24a551F3b954096").unwrap()), // Account 10
        Address::from_slice(&hex::decode("71bE63f3384f5fb98995898A86B02Fb2426c5788").unwrap()), // Account 11
        Address::from_slice(&hex::decode("FABB0ac9d68B0B445fB7357272Ff202C5651694a").unwrap()), // Account 12
        Address::from_slice(&hex::decode("1CBd3b2770909D4e10f157cABC84C7264073C9Ec").unwrap()), // Account 13
        Address::from_slice(&hex::decode("dF3e18d64BC6A983f673Ab319CCaE4f1a57C7097").unwrap()), // Account 14
        Address::from_slice(&hex::decode("cd3B766CCDd6AE721141F452C550Ca635964ce71").unwrap()), // Account 15
        Address::from_slice(&hex::decode("2546BcD3c84621e976D8185a91A922aE77ECEc30").unwrap()), // Account 16
        Address::from_slice(&hex::decode("bDA5747bFD65F08deb54cb465eB87D40e51B197E").unwrap()), // Account 17
        Address::from_slice(&hex::decode("dD2FD4581271e230360230F9337D5c0430Bf44C0").unwrap()), // Account 18
        Address::from_slice(&hex::decode("8626f6940E2eb28930eFb4CeF49B2d1F2C9C1199").unwrap()), // Account 19
    ];
    let provider = setup_eth_provider(Node::Anvil).await.unwrap();
    let block_id = BlockId::Number(BlockNumber::Latest);
    println!("{:?}", block_id);

    let fetcher = MerkleProofFetcher::new("http://localhost:8545", Some(provider)).unwrap();
    let slice_lengths = [1, 3, 5, 8, 10, 15, 20];
    let contract_address = Address::from_str("0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512").unwrap();

    for &length in &slice_lengths {
        if length <= user_owned_addresses.len() && length <= user_owned_private_keys.len() {
            let address_slice: Vec<Address> =
                user_owned_addresses.iter().take(length).cloned().collect();
            let key_slice: Vec<[u8; 32]> = user_owned_private_keys
                .iter()
                .take(length)
                .cloned()
                .collect();
            generate_defi_inputs(
                address_slice.clone().to_vec(),
                key_slice.clone().to_vec(),
                &fetcher,
                block_id,
                contract_address.clone(),
            )
            .await;
        } else {
            println!(
                "Warning: Requested length {} exceeds available accounts",
                length
            );
        }
    }
}
