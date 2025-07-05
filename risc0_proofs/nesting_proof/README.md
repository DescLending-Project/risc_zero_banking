
```bash
RISC0_USE_DOCKER=1 cargo build --release 
```
```bash
RISC0_USE_DOCKER=1 RISC0_DEV_MODE=1 cargo run -p host --bin host --release -- ../tradfi_score_tlsn_proof/receipt.bin ../account_merkel_proof/complete_receipt.bin
```
To extract imageID from receipt
```bash
cargo run -p host --bin extract_image_id --release -- ../account_merkle_proof/complete_receipt.bin
```