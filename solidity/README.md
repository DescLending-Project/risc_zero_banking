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
## Deploy with Foundry
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
Now you can go to score_publisher/



To test the nullifiers handling logic run the tests from NullifierTest.sol
```shell
forge test --match-contract NullifierTest -vv
```
