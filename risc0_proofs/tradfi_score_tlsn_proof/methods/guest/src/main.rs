#![no_std]
#![no_main]
extern crate alloc;
use alloc::{
    format, str,
    string::{String, ToString},
    vec::Vec,
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
    score: Option<u64>,
    user_id: Option<String>,
    date: Option<String>, // YYYY-MM-DD format
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

// Structs to parse the API response JSON
#[derive(Debug, Serialize, Deserialize)]
struct ApiResponse {
    data: ApiData,
    message: String,
    path: String,
    timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiData {
    score: ScoreData,
    #[serde(rename = "userId")]
    user_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ScoreData {
    value: u64,
}

fn main() {
    let proof_json: String = env::read();
    let mut output = VerificationOutput {
        is_valid: false,
        server_name: String::new(),
        score: None,
        user_id: None,
        date: None,
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
            "Key mismatch:\n embedded = {}\n expected = {}",
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

    // Extract structured data from transcript
    if let Some(transcript) = pres_out.transcript {
        if let Ok(response_text) = str::from_utf8(transcript.received_unsafe()) {
            // Try to parse the full JSON structure
            if let Ok(api_response) = serde_json::from_str::<ApiResponse>(response_text) {
                // Extract score
                output.score = Some(api_response.data.score.value);
                
                // Extract user ID
                output.user_id = Some(api_response.data.user_id);
                
                // Extract and format date (YYYY-MM-DD from timestamp)
                output.date = extract_date_from_timestamp(&api_response.timestamp);
            } else {
                // Fallback: try to extract score the old way
                if let Some(val) = response_text.split("value\":").nth(1) {
                    output.score = val
                        .chars()
                        .take_while(|c| c.is_ascii_digit())
                        .collect::<String>()
                        .parse()
                        .ok();
                }
                
                // Try to extract userId
                if let Some(user_part) = response_text.split("\"userId\":\"").nth(1) {
                    if let Some(user_id) = user_part.split("\"").next() {
                        output.user_id = Some(user_id.to_string());
                    }
                }
                
                // Try to extract timestamp
                if let Some(ts_part) = response_text.split("\"timestamp\":\"").nth(1) {
                    if let Some(timestamp) = ts_part.split("\"").next() {
                        output.date = extract_date_from_timestamp(timestamp);
                    }
                }
            }
        }
    }

    env::commit(&output);
}

// Extract YYYY-MM-DD from ISO timestamp like "2025-07-21T10:11:47.391983838Z"
fn extract_date_from_timestamp(timestamp: &str) -> Option<String> {
    if timestamp.len() >= 10 {
        Some(timestamp[0..10].to_string()) // Take first 10 chars: "2025-07-21"
    } else {
        None
    }
}