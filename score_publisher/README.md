
#Build and run publisher 

#First head over to Solidity/ to deploy the necessary contracts before you run these


```bash
RISC0_USE_DOCKER=1 cargo build --release
```
```bash
RISC0_USE_DOCKER=1 cargo run -p host --bin host --release -- \
  --first-receipt-path host/receipts/tradfi_tlsn_receipt.bin \
  --second-receipt-path host/receipts/eth_account_receipt.bin \
  --chain-id 31337 \
  --rpc-url http://localhost:8545 \
  --contract 0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512 \
  --eth-wallet-private-key 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
```