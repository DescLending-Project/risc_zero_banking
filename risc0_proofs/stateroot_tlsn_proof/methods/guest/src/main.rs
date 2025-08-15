#![no_std]
#![no_main]

extern crate alloc;
use alloc::{
    format, str,
    string::{String, ToString},
};

use bincode;
use hex;
use risc0_zkvm::guest::env;
use serde::{Deserialize, Serialize};
use tlsn_core::{
    presentation::{Presentation, PresentationOutput},
    signing::VerifyingKey as TlsnVerifyingKey,
    CryptoProvider,
};

risc0_zkvm::guest::entry!(main);

/// 33-byte compressed SEC-1 form of the Notary's public key
const EXPECTED_COMPRESSED_HEX: &str =
    "037b48f19c139b6888fb5e383a4d72c2335186fd5858e7ae743ab4bf8e071b06e7";

#[derive(Debug, Serialize, Deserialize)]
struct VerificationOutput {
    is_valid: bool,
    server_name: String,
    state_root: Option<String>,
    block_number: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct InputPresentationData {
    version: String,
    data: String,
    meta: MetaData,
}

#[derive(Debug, Serialize, Deserialize)]
struct MetaData {
    #[serde(rename = "notaryUrl")]
    notary_url: String,
    #[serde(rename = "websocketProxyUrl")]
    websocket_proxy_url: String,
}

fn main() {
    let start = env::cycle_count();
    let proof_json: String = env::read();

    let mut output = VerificationOutput {
        is_valid: false,
        server_name: String::new(),
        state_root: None,
        block_number: None,
        error: None,
    };

    // Parse outer JSON
    let input: InputPresentationData = match serde_json::from_str(&proof_json) {
        Ok(v) => v,
        Err(e) => {
            output.error = Some(format!("Failed to parse outer JSON: {}", e));
            env::commit(&output);
            return;
        }
    };

    // Hex-decode bincode payload
    let proof_bytes = match hex::decode(&input.data) {
        Ok(b) => b,
        Err(e) => {
            output.error = Some(format!("Failed to hex-decode data: {}", e));
            env::commit(&output);
            return;
        }
    };

    // Bincode-deserialize into Presentation
    let tlsn_presentation: Presentation = match bincode::deserialize(&proof_bytes) {
        Ok(p) => p,
        Err(e) => {
            output.error = Some(format!("Bincode deserialize failed: {}", e));
            env::commit(&output);
            return;
        }
    };

    // Key check: compare compressed form directly
    let embedded_vk: &TlsnVerifyingKey = tlsn_presentation.verifying_key();
    let embedded_hex = hex::encode(&embedded_vk.data);
    if embedded_hex != EXPECTED_COMPRESSED_HEX {
        output.error = Some(format!(
            "Key mismatch:\n  embedded = {}\n  expected = {}",
            embedded_hex, EXPECTED_COMPRESSED_HEX,
        ));
        env::commit(&output);
        return;
    }

    // All checks passed: verify Presentation
    let provider = CryptoProvider::default();
    let pres_out: PresentationOutput = match tlsn_presentation.verify(&provider) {
        Ok(o) => o,
        Err(e) => {
            output.error = Some(format!("Presentation.verify() failed: {:?}", e));
            env::commit(&output);
            return;
        }
    };

    // Extract server_name
    if let Some(sn) = pres_out.server_name {
        output.server_name = sn.to_string();
    }
    output.is_valid = true;

    // Extract stateRoot and block number from Alchemy API response
    if let Some(transcript) = pres_out.transcript {
        if let Ok(s) = str::from_utf8(transcript.received_unsafe()) {
            // Extract stateRoot
            if let Some(state_root_start) = s.find("\"stateRoot\":\"") {
                let start_pos = state_root_start + 13; // Length of "stateRoot":""
                if let Some(end_pos) = s[start_pos..].find("\"") {
                    output.state_root = Some(s[start_pos..start_pos + end_pos].to_string());
                }
            }

            // Extract block number
            if let Some(number_start) = s.find("\"number\":\"") {
                let start_pos = number_start + 10; // Length of "number":""
                if let Some(end_pos) = s[start_pos..].find("\"") {
                    output.block_number = Some(s[start_pos..start_pos + end_pos].to_string());
                }
            }
        }
    }

    // Validate that we extracted both values
    if output.state_root.is_none() || output.block_number.is_none() {
        output.error =
            Some("Failed to extract stateRoot or block number from response".to_string());
        output.is_valid = false;
    }

    env::commit(&output);
    let total_cycles = env::cycle_count() - start;
    env::log(&format!("{}: Total Cycyles", total_cycles));
}

