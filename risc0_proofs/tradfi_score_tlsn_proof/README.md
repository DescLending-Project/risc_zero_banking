# RISC0-TLSN Verifier for Fetched TradFi Score

Verify TLSNotary proofs of HTTPS session integrity inside the RISC0 zkVM to validate traditional credit scores fetched from external APIs.

### Directory Structure
```text
├── data/           # TLSN proof files 
├── host/           # RISC0 host that executes the zkVM
└── methods/guest/  # zkVM guest code with TLSN proof verification logic
```

### Input/Output

**Input:**
- `proof.json` - TLSNotary proof file containing the authenticated HTTPS session data

**Output:**
- `is_valid: bool` - Whether the TLSN proof is cryptographically valid
- `server_name: String` - Domain name of the verified server
- `score: Option<u64>` - Extracted credit score 
- `user_id_hash: Option<String>` - Hashed user identifier
- `tradfi_date_timestamp: Option<u64>` - Unix Timestamp of when the score was fetched
- `error: Option<String>` - Error message if verification fails

## Prerequisites

Install the following:
- **Rust** 
- **RISC Zero toolchain**
- **LLD** linker

## Setup

1. Make the linker script executable:
   ```bash
   chmod +x riscv32im-linker.sh
   ```
2. Export the linker environment variable:
   ```bash
   export HOST_LINKER="$PWD/riscv32im-linker.sh"
   ```
## Building
Use Docker to build your guest code with the RISC0 toolchain:
```bash
RISC0_USE_DOCKER=1 \
  CARGO_TARGET_RISCV32IM_RISC0_ZKVM_ELF_LINKER="$HOST_LINKER" \
  cargo build --workspace --release
```


This will compile both the guest (zkVM) and host binaries under target/release.

## Running in Development Mode (fast)
To skip the extensive proof generation run quickly in dev mode:
```bash
RISC0_USE_DOCKER=1 \
RISC0_DEV_MODE=1 \
cargo run -p host --release -- data/proofHighScore.json
```
- data/proof.json path to TLSNotary proof to be verified.


