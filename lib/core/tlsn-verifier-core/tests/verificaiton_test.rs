use std::fs;
use tlsn_verifier_core::verify_tlsn_presentation;

#[test]
fn test_valid_presentation() {
    // Read test data
    let presentation_json = fs::read_to_string("../../test_data/tlsn/valid_presentation.json")
        .expect("Test data not found");

    let result = verify_tlsn_presentation(&presentation_json);

    assert!(result.verified, "Expected verification to succeed");
    assert!(result.error.is_none(), "Expected no errors");
    // Add more assertions based on expected content
}

#[test]
fn test_invalid_presentation() {
    // Test with invalid data
    let presentation_json = r#"{"presentationJson":{"version":"0.1.0","data":"invalidhex"}}"#;

    let result = verify_tlsn_presentation(&presentation_json);

    assert!(!result.verified, "Expected verification to fail");
    assert!(result.error.is_some(), "Expected an error");
}
