use std::fs;
use tlsn_verifier_core::verify_tlsn_presentation;

fn main() {
    // Read the presentation JSON
    let presentation_json = fs::read_to_string("../test_data/valid_presentation.json")
        .expect("Failed to read presentation.json file");

    // Verify the presentation
    let result = verify_tlsn_presentation(&presentation_json);

    // Print results
    println!("=== TLS NOTARY VERIFICATION RESULT ===");
    println!("Verification Status: {}", result.verified);

    if let Some(error) = result.error {
        println!("Error: {}", error);
        return;
    }

    // ... Print other result fields
}
