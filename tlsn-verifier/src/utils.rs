use crate::types::{KeyMaterial};
use sha2::{Digest, Sha512};

/// Encodes a UTF-8 message string into its hexadecimal representation
pub fn encode_message_hex(
    message: &str,
) -> String {
    return hex::encode(message)
}

/// Signs a hex-encoded message string using the provided `KeyMaterial`
/// and returns the signature as a hex string
pub fn sign_message(
    key_material: &KeyMaterial,
    message_hex: &str,
) -> String {
    println!("[sign_message] Signing message: {}", message_hex);
    let signature = key_material.sign_message(message_hex.as_bytes());
    let signature_bytes = signature.to_bytes();
    let signature_hex_encoded = hex::encode(signature_bytes);
    println!("[sign_message] Signature generated: {}", signature_hex_encoded);
    return signature_hex_encoded;
}

/// Computes a report hash (SHA-512) of the public key to embed in attestation
pub fn prepare_report_data(
    data : &str,
) -> String {
    // Convert the input data to a SHA-512 hash
    let hash = Sha512::digest(data.as_bytes());
    println!("[prepare_report_data] SHA-512 hash computed: {}", hex::encode(hash));
    // Return the hex-encoded hash as a string
    format!("0x{}", hex::encode(hash))
}