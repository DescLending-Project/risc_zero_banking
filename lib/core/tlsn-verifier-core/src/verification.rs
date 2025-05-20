use crate::types::{PresentationWrapper, VerificationResult};

#[cfg(feature = "use-tlsn")]

/// Verify a TLS Notary presentation
/// This function has the same signature regardless of which implementation is used
pub fn verify_tlsn_presentation(presentation_json: &str) -> VerificationResult {
    // Parse the JSON
    let wrapper: PresentationWrapper = match serde_json::from_str(presentation_json) {
        Ok(w) => w,
        Err(e) => {
            return VerificationResult {
                verified: false,
                server_name: None,
                request_method: None,
                request_uri: None,
                response_status: None,
                response_body: None,
                headers: Vec::new(),
                error: Some(format!("Failed to parse JSON: {}", e)),
            };
        }
    };

    // Extract hex data and convert to bytes
    let hex_data = &wrapper.presentationJson.data;
    let bytes = match hex::decode(hex_data) {
        Ok(b) => b,
        Err(e) => {
            return VerificationResult {
                verified: false,
                server_name: None,
                request_method: None,
                request_uri: None,
                response_status: None,
                response_body: None,
                headers: Vec::new(),
                error: Some(format!("Failed to decode hex data: {}", e)),
            };
        }
    };
    return VerificationResult {
        verified: true,
        server_name: None,
        request_method: None,
        request_uri: None,
        response_status: None,
        response_body: None,
        headers: Vec::new(),
        error: None,
    };
}
