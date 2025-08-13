use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use regex::Regex;
use std::time::Instant;
use tlsn_core::CryptoProvider;

use crate::config;
use crate::types::{PresentationJSON, TLSNVerificationResult, VerificationError};

/// Verifies a TLSNotary presentation proof from JSON string input
///
/// # Arguments
///
/// * `json` - A string slice containing a TLSNotary presentation in JSON format.
///
/// # Returns
///
/// * `Ok(VerificationResult)` if the proof is valid and passes all checks
/// * `Err(VerificationError)` if any verification step fails
pub fn verify_tlsn_proof(
    presentation_json: &PresentationJSON,
) -> Result<TLSNVerificationResult, VerificationError> {
    let total_start = Instant::now(); // Track total verification time

    println!("[{}] ⏱ Starting verification...", chrono::Utc::now());

    // Step 1: Parse JSON into PresentationJSON struct
    let start = Instant::now();

    // Step 2: Check for expected TLSNotary core version
    let expected_version = config::get_tlsn_core_version();
    if presentation_json.version != expected_version {
        return Err(VerificationError {
            message: format!(
                "Version mismatch: expected '{}', got '{}'",
                expected_version, presentation_json.version
            ),
        });
    }

    // Step 3: Convert presentation_json -> Presentation object
    let start = Instant::now();
    let presentation = presentation_json
        .to_presentation()
        .map_err(|e| VerificationError {
            message: format!("Invalid presentation encoding: {}", e),
        })?;
    println!("✅ Presentation decoded in {:?}", start.elapsed());

    // Step 4: Ensure verifying key exists
    let verifying_key = presentation.verifying_key().data.clone();
    if verifying_key.is_empty() {
        return Err(VerificationError {
            message: "Verifying key is empty or missing".to_string(),
        });
    }

    // Step 5: Run cryptographic verification of the presentation
    let start = Instant::now();
    let pres_out = presentation
        .verify(&CryptoProvider::default())
        .map_err(|e| VerificationError {
            message: format!("Presentation verification failed: {}", e),
        })?;
    println!("✅ Presentation verified in {:?}", start.elapsed());

    // Step 7: Parse timestamp from connection info
    let secs = pres_out.connection_info.time as i64;
    let naive = NaiveDateTime::from_timestamp_opt(secs, 0).ok_or_else(|| VerificationError {
        message: "Invalid or missing timestamp".to_string(),
    })?;
    let dt: DateTime<Utc> = Utc.from_utc_datetime(&naive);

    // Step 8: Extract transcript and get sent/received messages
    let mut transcript = pres_out.transcript.ok_or_else(|| VerificationError {
        message: "Missing transcript in presentation output".to_string(),
    })?;

    transcript.set_unauthed(b'X'); // Mark unauthenticated region
    let sent_bytes = transcript.sent_unsafe().to_vec();
    let recv_bytes = transcript.received_unsafe().to_vec();
    let sent = String::from_utf8_lossy(&sent_bytes);
    let recv = String::from_utf8_lossy(&recv_bytes);

    println!(
        "✅ Transcript parsed, sent/recv size = {}/{}",
        sent_bytes.len(),
        recv_bytes.len()
    );

    // Step 9: Extract and validate Host header
    let host_line = sent
        .lines()
        .find(|line| line.to_lowercase().starts_with("host:"))
        .ok_or_else(|| VerificationError {
            message: "Missing 'Host' header in sent transcript".to_string(),
        })?;
    let host = host_line.trim_start_matches("host:").trim();

    println!("✅ Host header extracted: {}", host);

    println!("✅ Verification complete in {:?}", total_start.elapsed());

    // Step 12: Return result with useful metadata
    Ok(TLSNVerificationResult {
        is_valid: true,
        server_name: host.to_string(),
        verifying_key: hex::encode(verifying_key),
        sent_hex_encoded: hex::encode(&sent_bytes),
        sent_readable: sent.to_string(),
        recv_hex_encoded: hex::encode(&recv_bytes),
        recv_readable: recv.to_string(),
        time: dt.timestamp() as u64,
    })
}

pub fn extract_tradfi_score(
    verification_result: &TLSNVerificationResult
) -> Result<u64, VerificationError> {
    let request_line = verification_result.sent_readable.lines().next().ok_or_else(|| VerificationError {
        message: "Missing request line in sent transcript".to_string(),
    })?;

    let path_regex = Regex::new(
        r#"GET\s+(?:https?://[^/]+)?(/users/[^/]+/credit-score)\s+HTTP/1\.1"#,
    )
    .map_err(|e| VerificationError {
        message: format!("Regex compilation failed: {}", e),
    })?;

    let _path = path_regex
        .captures(request_line)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str())
        .ok_or_else(|| VerificationError {
            message: "Request path is missing or invalid".to_string(),
        })?;

    let score_regex = Regex::new(r#""value"\s*:\s*(\d+)"#).map_err(|e| VerificationError {
        message: format!("Regex compilation failed: {}", e),
    })?;

    let creedit_score = score_regex
        .captures(verification_result.recv_readable.as_str())
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().parse::<u64>().unwrap())
        .ok_or_else(|| VerificationError {
            message: "Credit score value is missing from response".to_string(),
        })?;

    return Ok(credit_score);
}

/*
Example state proof readable response
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "hash": "0xe895d4e72ac901cc88aafdd8ef47bf8f6c41d5fd16d2f116fe3f4649e329daac",
    "parentHash": "0xa127c5f6854b1d87c932a20337fb07483dfa33e35391d09e4d28acca23568081",
    "sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
    "miner": "0x0000000000000000000000000000000000000000",
    "stateRoot": "0x95d501fe33a79a591e545b832ebf4f7721e0127c4a02647e53aab028ac69f384",
    "transactionsRoot": "0x53766e3c3dc859a8a7b330499a394e4bd9071963a03c0954491032fdcc549e11",
    "receiptsRoot": "0xd7193ccba565faf70090f581b424b5d0e997cb1510ba40dd6f5d101021b40500",
    "logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
    "difficulty": "0x0",
    "number": "0x3",
    "gasLimit": "0x1c9c380",
    "gasUsed": "0x27b9ce",
    "timestamp": "0x68933f6c",
    "extraData": "0x",
    "mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
    "nonce": "0x0000000000000000",
    "baseFeePerGas": "0x2e9094ea",
    "withdrawalsRoot": "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
    "blobGasUsed": "0x0",
    "excessBlobGas": "0x0",
    "parentBeaconBlockRoot": "0x0000000000000000000000000000000000000000000000000000000000000000",
    "totalDifficulty": "0x0",
    "size": "0x32d6",
    "uncles": [],
    "transactions": [
      "0x2278d196fb78ed9a89fbf7dd477529c2c366899ccfc95754465b4f6cbe570b01"
    ],
    "withdrawals": []
  }
}
*/

pub fn extract_state_root(
    verification_result: &TLSNVerificationResult
) -> Result<String, VerificationError> {
    let state_root = verification_result.recv_readable.lines().find_map(|line| {
        let re = Regex::new(r#""stateRoot"\s*:\s*"([^"]+)""#).unwrap();
        re.captures(line).and_then(|cap| cap.get(1).map(|m| m.as_str().to_string()))
    }).ok_or_else(|| VerificationError {
        message: "State root is missing from response".to_string(),
    })?;

    Ok(state_root)
}

pub fn extract_block_number(
    verification_result: &TLSNVerificationResult
) -> Result<u64, VerificationError> {
    let block_number = verification_result.recv_readable.lines().find_map(|line| {
        let re = Regex::new(r#""number"\s*:\s*"(\d+)"#).unwrap();
        re.captures(line).and_then(|cap| cap.get(1).map(|m| m.as_str().parse::<u64>().unwrap()))
    }).ok_or_else(|| VerificationError {
        message: "Block number is missing from response".to_string(),
    })?;

    Ok(block_number)
}

pub fn extract_time_stamp(
    verification_result: &TLSNVerificationResult
) -> Result<u64, VerificationError> {
    let time_stamp = verification_result.recv_readable.lines().find_map(|line| {
        let re = Regex::new(r#""timestamp"\s*:\s*"(\d+)"#).unwrap();
        re.captures(line).and_then(|cap| cap.get(1).map(|m| m.as_str().parse::<u64>().unwrap()))
    }).ok_or_else(|| VerificationError {
        message: "Timestamp is missing from response".to_string(),
    })?;

    Ok(time_stamp)
}