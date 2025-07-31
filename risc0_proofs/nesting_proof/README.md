
```bash
RISC0_USE_DOCKER=1 cargo build --release 
```
```bash
RISC0_USE_DOCKER=1 RISC0_DEV_MODE=1 cargo run -p host --bin host --release -- receipts/receipt.bin receipts/test_receipt.bin
```
To extract imageID from receipt
```bash
cargo run -p host --bin extract_image_id --release -- receipts/receipt.bin
```