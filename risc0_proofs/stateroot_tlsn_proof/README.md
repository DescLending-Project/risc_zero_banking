# RISC0-TLSN Verifier for Fetched Stateroot

Verify TLSNotary proofs of HTTPS session integrity inside the RISC0 zkVM to validate blockchain state roots fetched from external RPC APIs.

### Directory Structure
```text
├── data/           # TLSN proof files 
├── host/           # RISC0 host that executes the zkVM
└── methods/guest/  # zkVM guest code with TLSN proof verification logic
```
### Decoded TLSN Proof Structure
The TLSN proof contains a verified JSON-RPC response from a blockchain RPC endpoint. Example of the decoded data structure:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "hash": "0xe895d4e72ac901cc88aafdd8ef47bf8f6c41d5fd16d2f116fe3f4649e329daac",
    "parentHash": "0xa127c5f6854b1d87c932a20337fb07483dfa33e35391d09e4d28acca23568081",
    "stateRoot": "0x95d501fe33a79a591e545b832ebf4f7721e0127c4a02647e53aab028ac69f384",
    "transactionsRoot": "0x53766e3c3dc859a8a7b330499a394e4bd9071963a03c0954491032fdcc549e11",
    "receiptsRoot": "0xd7193ccba565faf70090f581b424b5d0e997cb1510ba40dd6f5d101021b40500",
    "number": "0x3",
    "gasLimit": "0x1c9c380",
    "gasUsed": "0x27b9ce",
    "timestamp": "0x68933f6c",
    "totalDifficulty": "0x0",
    "size": "0x32d6",
    "transactions": [
      "0x2278d196fb78ed9a89fbf7dd477529c2c366899ccfc95754465b4f6cbe570b01"
    ]
  }
}
```
### Input/Output

**Input:**
- `proof.json` - TLSNotary proof file containing the authenticated HTTPS session data

**Output:**
- `is_valid: bool` - Whether the TLSN proof is cryptographically valid
- `server_name: String` - Domain name of the verified server
- `state_root: Option<String>` - Extracted stateroot
- `block_number: Option<String>` - Block number of the verified state
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
cargo run -p host --release -- data/stateroot_proof.json
```
- data/proof.json path to TLSNotary proof to be verified.


