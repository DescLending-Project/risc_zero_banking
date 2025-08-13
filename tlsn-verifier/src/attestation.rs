// Imports the key material and utility types for attestation
use crate::key_manager::try_get_key_material;
use crate::tappd_service;
use crate::types::*;
use crate::types::{AttestationError, SignedAttestation};
use crate::utils;
use serde_json::json;

/// Connects to the TDX quote provider (`tappd`) via Unix socket,
/// sends a custom attestation request with the report_data derived from the key,
/// and returns the parsed attestation quote as a `GetQuoteResponse`
pub async fn read_attestation_report(data: &str) -> Result<GetQuoteResponse, AttestationError> {
    // Construct the evidence
    let custom_evidence = json!({
        "report_data": data,  // 64-byte SHA512 hash (hex)
        "hash_algorithm": "raw"  // Request raw hashing algorithm
    });
    println!(
        "[read_attestation_report] Custom evidence constructed: {}",
        custom_evidence
    );

    // Send the request to the tappd socket and await response
    let res = tappd_service::send_quote_request(&custom_evidence.to_string())
        .await
        .map_err(|e| AttestationError {
            message: format!("Tappd Service Error: {}", e.message),
        })?;
    println!("[read_attestation_report] Response received from tappd service");

    // Read the response body bytes
    let body_bytes =
        hyper::body::to_bytes(res.into_body())
            .await
            .map_err(|e| AttestationError {
                message: format!("Failed to read response body: {}", e),
            })?;
    println!("[read_attestation_report] Response body read successfully");

    // Parse the body into a `GetQuoteResponse` structure
    let parsed: GetQuoteResponse =
        serde_json::from_slice(&body_bytes).map_err(|e| AttestationError {
            message: format!("Failed to parse GetQuoteResponse: {}", e),
        })?;
    println!("[read_attestation_report] GetQuoteResponse parsed successfully");
    Ok(parsed)
}

/// Combines the attestation report with a digital signature and verifying key
/// to create a `SignedAttestation` which can be sent for remote verification
pub async fn get_attestation_report_with_signature(
    data: &str,
) -> Result<SignedAttestation, AttestationError> {
    // Ensure key material is available
    let key_material = try_get_key_material().ok_or_else(|| AttestationError {
        message: "Key material not initialized".to_string(),
    })?;
    println!("[get_attestation_report_with_signature] Key material initialized successfully");

    // Fetch the attestation report from tappd
    let report = read_attestation_report(data).await?;
    println!("[get_attestation_report_with_signature] Attestation report fetched successfully");
    let report_data = report.quote;
    println!(
        "[get_attestation_report_with_signature] Report data: {}",
        report_data
    );

    // Convert the report data to hex so it can be signed
    let report_data_hex: String = utils::encode_message_hex(&report_data);
    println!(
        "[get_attestation_report_with_signature] Report data hex: {}",
        report_data_hex
    );

    // Sign the hex-encoded attestation report
    let signature = utils::sign_message(&key_material, &report_data_hex);
    println!(
        "[get_attestation_report_with_signature] Signature generated successfully: {}",
        signature
    );

    // Get the verifying key in hex format
    let encoded_key = key_material.encode_verify_key();
    println!(
        "[get_attestation_report_with_signature] Verifying key encoded successfully: {}",
        encoded_key
    );
    // Construct the signed attestation payload
    Ok(SignedAttestation {
        quote: report_data,                     // Raw quote data (still hex)
        signature_hex_encoded: signature,       // Signature over quote
        verifying_key_hex_encoded: encoded_key, // Public key used to sign
        verifying_key_certificate_chain: key_material.certificate_chain.clone(), // Optional certificate chain
    })
}
