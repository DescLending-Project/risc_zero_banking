# Solidity Smart Contracts

Smart contracts for RISC Zero proof verification and credit score management on Ethereum.

## Directory Structure
```text
├── contracts/
│   ├── risc0/                    # RISC Zero verification contracts
│   │   ├── ImageID.sol           # Circuit binding for zkVM image verification
│   │   └── ...                   # Other R0 verification utilities
│   ├── CreditScore.sol           # Main contract for score proof upload and management
│   └── Lending.sol               # Mock lending contract for testing integrations
├── deploy/
│   └── Deploy.s.sol              # Deployment scripts
├── test/
│   └── NullifierTest.sol         # Tests for nullifier handling logic
└── lib/                          # Dependencies (installed as git submodules)
```


## Install required Libs 
```shell
git submodule add https://github.com/OpenZeppelin/openzeppelin-contracts
lib/openzeppelin-contracts

git submodule add https://github.com/foundry-rs/forge-std
lib/forge-std

git submodule add https://github.com/risc0/risc0-ethereum
lib/risc0-ethereum
```
## Deploy with Foundry
First, start Anvil in a separte Shell to get the env vars
```shell
anvil
```
copy the env vars from anvil and export in your other shell
```shell
export CONTRACT_ADDRESS=0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512
export CHAIN_ID=31337
export RPC_URL=http://localhost:8545
export ETH_WALLET_PRIVATE_KEY=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
```

or source the variables from .local_env file
```shell
source .local_env
```

Build Contracts
```shell
forge build
```
Deploy Contracts
```shell
forge script deploy/Deploy.s.sol:CreditScoreDeploy --rpc-url $RPC_URL --broadcast --private-key $ETH_WALLET_PRIVATE_KEY
```
Now you can go to score_publisher/



To test the nullifiers handling logic run the tests from NullifierTest.sol
```shell
forge test --match-contract NullifierTest -vv
```
