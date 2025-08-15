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
use sha3::{Digest, Keccak256};
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
    user_id_hash: Option<String>,
    tradfi_date_timestamp: Option<u64>,
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
    let start = env::cycle_count();
    let proof_json: String = env::read();
    let mut output = VerificationOutput {
        is_valid: false,
        server_name: String::new(),
        score: None,
        user_id_hash: None,
        tradfi_date_timestamp: None,
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

                // Hash user ID for privacy
                output.user_id_hash = Some(hash_user_id(&api_response.data.user_id));

                // Convert timestamp to date-only unix timestamp
                output.tradfi_date_timestamp =
                    parse_date_to_unix_timestamp(&api_response.timestamp);

                // Convert timestamp to unix timestamp
                output.tradfi_date_timestamp =
                    parse_date_to_unix_timestamp(&api_response.timestamp);
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

                // Try to extract and hash userId
                if let Some(user_part) = response_text.split("\"userId\":\"").nth(1) {
                    if let Some(user_id) = user_part.split("\"").next() {
                        output.user_id_hash = Some(hash_user_id(user_id));
                    }
                }

                // Try to extract timestamp and convert to date-only
                if let Some(ts_part) = response_text.split("\"timestamp\":\"").nth(1) {
                    if let Some(timestamp) = ts_part.split("\"").next() {
                        output.tradfi_date_timestamp = parse_date_to_unix_timestamp(timestamp);
                    }
                }
            }
        }
    }

    env::commit(&output);
    let total_cycles = env::cycle_count() - start;
    env::log(&format!("{}: Total Cycyles", total_cycles));
}

// Hash user ID with Keccak256 for privacy - using RISC Zero optimized precompile
fn hash_user_id(user_id: &str) -> String {
    let mut hasher = Keccak256::new();
    hasher.update(user_id.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

// Parse date part to Unix timestamp (date only, 00:00:00 UTC)
// Input: "2025-07-21T10:11:47.391983838Z"
// Output: Unix timestamp for "2025-07-21 00:00:00 UTC"
fn parse_date_to_unix_timestamp(iso_timestamp: &str) -> Option<u64> {
    if iso_timestamp.len() < 10 {
        return None;
    }

    let date_part = &iso_timestamp[0..10]; // "2025-07-21"
    let mut date_parts = date_part.split('-');

    let year: u32 = date_parts.next()?.parse().ok()?;
    let month: u32 = date_parts.next()?.parse().ok()?;
    let day: u32 = date_parts.next()?.parse().ok()?;

    // Validate reasonable ranges
    if year < 1970 || year > 2100 || month < 1 || month > 12 || day < 1 || day > 31 {
        return None;
    }

    // Days since Unix epoch (Jan 1, 1970)
    let mut days_since_epoch = 0u64;

    // Add days for complete years
    for y in 1970..year {
        days_since_epoch += if is_leap_year(y) { 366 } else { 365 };
    }

    // Days in each month
    let mut days_in_months = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    if is_leap_year(year) {
        days_in_months[1] = 29; // February in leap year
    }

    // Add days for complete months in current year
    for m in 1..month {
        if m <= 12 {
            days_since_epoch += days_in_months[(m - 1) as usize] as u64;
        }
    }

    // Add remaining days (subtract 1 because day 1 of month = 0 additional days)
    days_since_epoch += (day - 1) as u64;

    // Convert days to seconds (date at 00:00:00 UTC)
    Some(days_since_epoch * 24 * 60 * 60)
}

fn is_leap_year(year: u32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

