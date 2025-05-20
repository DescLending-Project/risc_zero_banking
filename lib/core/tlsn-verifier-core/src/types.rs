use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresentationWrapper {
    pub presentationJson: PresentationJson,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresentationJson {
    pub version: String,
    pub data: String,
    pub meta: Meta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    pub notaryUrl: String,
    pub websocketProxyUrl: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub verified: bool,
    pub server_name: Option<String>,
    pub request_method: Option<String>,
    pub request_uri: Option<String>,
    pub response_status: Option<u16>,
    pub response_body: Option<String>,
    pub headers: Vec<(String, String)>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpMessage {
    pub method: Option<String>,
    pub uri: Option<String>,
    pub status: Option<u16>,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}
