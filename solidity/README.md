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
git submodule update --init --recursive
```
or
```shell
forge build
```
## Deploy locally with Foundry 
First, start Anvil in a separte Shell to get the env vars
```shell
anvil
```

Source the variables from .local_env file
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

Now you can go to [score publisher](../score_publisher/)

## Deploy to Sepolia 

Export your Sepolia wallet private key and Alchemy (Sepolia) API key:
```shell
export ETH_WALLET_PRIVATE_KEY=""
export ALCHEMY_API_KEY=""
```
and then deploy the contracts:
```shell
forge script script/Deploy.s.sol --rpc-url https://eth-sepolia.g.alchemy.com/v2/${ALCHEMY_API_KEY:?} --broadcast
```

## Nullifier Tests
To test the nullifiers handling logic run the tests from NullifierTest.sol
```shell
forge test --match-contract NullifierTest -vv
```