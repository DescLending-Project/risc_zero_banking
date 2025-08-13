// Core serialization and crypto utilities
use bincode;
use k256::pkcs8::der;
use serde::{Deserialize, Serialize};
use tlsn_core::presentation::Presentation;
use anyhow::Result;
use hex::FromHexError;
use serde_json::{Value, from_str};
use hex;
use p256::{
    EncodedPoint,
    ecdsa::{Signature, VerifyingKey, SigningKey, signature::Signer},
};

use p256::pkcs8::DecodePrivateKey;

use rand_core::OsRng;
use sha2::{Digest, Sha512};
/// Represents a TLSNotary presentation in JSON form, including version info, data payload, and metadata.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PresentationJSON {
    pub version: String,  // Version of the presentation format
    pub data: String,     // Hex-encoded serialized Presentation
    pub meta: Meta,       // Additional metadata such as notary URL
}

/// Metadata associated with a presentation
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    pub notary_url: String,                    // URL of the notary service
    pub websocket_proxy_url: Option<String>,   // Optional proxy for WebSocket connections
}

/// Structure containing the result of a successful verification
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TLSNVerificationResult {
    pub is_valid: bool,                    // Indicates if the presentation is valid
    pub server_name: String,               // Verified TLS server name
    pub verifying_key: String,             // Hex-encoded verifying key
    pub sent_hex_encoded: String,          // Hex-encoded sent message
    pub sent_readable: String,             // Human-readable sent message
    pub recv_hex_encoded: String,          // Hex-encoded received message
    pub recv_readable: String,             // Human-readable received message
    pub time: u64,                         // Timestamp of verification
}



/// Error that occurred during the verification process
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct VerificationError {
    pub message: String,                   // Human-readable error message
}

// Allows converting any Display-able error into a VerificationError
impl<E: std::fmt::Display> From<E> for VerificationError {
    fn from(e: E) -> Self {
        VerificationError {
            message: e.to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TappdError {
    pub message: String,                   // Human-readable error message from Tappd
}

impl<E: std::fmt::Display> From<E> for TappdError {
    fn from(e: E) -> Self {
        TappdError {
            message: e.to_string(),
        }
    }
}

/// Error that occurred during attestation
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AttestationError {
    pub message: String,
}

// Conversion implementation for AttestationError from any displayable error
impl<E: std::fmt::Display> From<E> for AttestationError {
    fn from(e: E) -> Self {
        AttestationError {
            message: e.to_string(),
        }
    }
}

/// Error structure for key management operations
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct KeyManagerError {
    pub message: String,
}

// Allows any error implementing Display to be wrapped in KeyManagerError
impl<E: std::fmt::Display> From<E> for KeyManagerError {
    fn from(e: E) -> Self {
        KeyManagerError {
            message: e.to_string(),
        }
    }
}

/// Wrapper for both verification result and attestation output
#[derive(Deserialize, Serialize)]
pub struct VerificationResponse {
    pub verification: Result<u16, VerificationError>, // Result of verification process
    pub attestation: Result<SignedAttestation, AttestationError>,    // Result of attestation (with signature)
}

/// Resulting signed attestation after successful proof
#[derive(Deserialize, Serialize)]
pub struct SignedAttestation {
    pub quote: String,                                // Hex-encoded attestation quote
    pub signature_hex_encoded: String,                // Hex-encoded signature over the attestation
    pub verifying_key_hex_encoded: String,            // Verifying key used to generate the signature
    pub verifying_key_certificate_chain: Option<Vec<String>>, // Optional PEM certificate chain
}

impl PresentationJSON {
    /// Parses a PresentationJSON from a JSON string
    pub fn from_json_str(json: &str) -> Result<Self, serde_json::Error> {
        return serde_json::from_str(json);
    }

    /// Decodes the presentation hex string into a Presentation struct
    pub fn to_presentation(&self) -> Result<Presentation, Box<dyn std::error::Error>> {
        let tmp_data: String = self.data.chars().filter(|c| !c.is_whitespace()).collect();
        let raw = hex::decode(&tmp_data)?;
        let presentation: Presentation = bincode::deserialize(&raw)?;
        Ok(presentation)
    }
}

/// Represents a single entry in the attestation event log
#[derive(Serialize, Deserialize)]
pub struct EventLog {
    pub imr: u32,               // Integrity Measurement Register index
    pub event_type: u32,        // Numeric event type
    pub digest: String,         // Hex-encoded digest of the event
    pub event: String,          // Event type string
    pub event_payload: String,  // Associated payload as a string
}

/// Response containing a derived key and its associated certificate chain
#[derive(Serialize, Deserialize)]
pub struct GetKeyResponse {
    pub key: String,                        // PEM or hex-encoded private key
    pub certificate_chain: Vec<String>,    // Chain of PEM-encoded certificates
}

impl GetKeyResponse {
    /// Decodes the key from hex to bytes
    pub fn decode_key(&self) -> Result<Vec<u8>, FromHexError> {
        hex::decode(&self.key)
    }

    /// Decodes the certificate chain from hex to bytes
    pub fn decode_certificate_chain(&self) -> Result<Vec<Vec<u8>>, FromHexError> {
        self.certificate_chain.iter().map(hex::decode).collect()
    }
}

/// Response containing a quote and associated event log (for attestation)
#[derive(Serialize, Deserialize)]
pub struct GetQuoteResponse {
    pub quote: String,         // Hex-encoded quote
    pub event_log: String,     // JSON-encoded event log
}

impl GetQuoteResponse {
    /// Decode the attestation quote
    pub fn decode_quote(&self) -> Result<Vec<u8>, FromHexError> {
        hex::decode(&self.quote)
    }

    /// Parse the event log JSON into a list of EventLog entries
    pub fn decode_event_log(&self) -> Result<Vec<EventLog>, serde_json::Error> {
        serde_json::from_str(&self.event_log)
    }
}

/// Response structure for metadata/info endpoint, including instance details and security state
#[derive(Serialize, Deserialize)]
pub struct InfoResponse {
    pub app_id: String,              // Application ID
    pub instance_id: String,         // Unique instance ID
    pub app_cert: String,            // Application certificate (PEM)
    pub tcb_info: TcbInfo,           // Trusted Computing Base measurements
    pub app_name: String,            // Application name
    pub public_logs: bool,           // Whether logs are publicly visible
    pub public_sysinfo: bool,        // Whether system info is publicly visible
    pub device_id: String,           // Device identifier
    pub mr_aggregated: String,       // Aggregated measurement hash
    pub os_image_hash: String,       // OS image measurement
    pub key_provider_info: String,   // Description of key provider used
    pub compose_hash: String,        // Docker Compose file hash
}

impl InfoResponse {
    /// Handles the case where `tcb_info` is embedded as a JSON string instead of a structured object
    pub fn validated_from_value(mut obj: Value) -> Result<Self, serde_json::Error> {
        if let Some(tcb_info_str) = obj.get("tcb_info").and_then(Value::as_str) {
            let parsed_tcb_info: TcbInfo = from_str(tcb_info_str)?;
            obj["tcb_info"] = serde_json::to_value(parsed_tcb_info)?;
        }
        serde_json::from_value(obj)
    }
}

/// Represents TCB (Trusted Computing Base) measurements
#[derive(Serialize, Deserialize)]
pub struct TcbInfo {
    pub mrtd: String,          // Measurement root of trust
    pub rootfs_hash: String,   // Filesystem root hash
    pub rtmr0: String,         // Runtime Measurement Register 0
    pub rtmr1: String,         // Runtime Measurement Register 1
    pub rtmr2: String,         // Runtime Measurement Register 2
    pub rtmr3: String,         // Runtime Measurement Register 3
    pub event_log: Vec<EventLog>,  // Related attestation events
}



pub struct KeyMaterial {
    pub signing_key: SigningKey,
    pub source: KeySource,
    pub certificate_chain: Option<Vec<String>>, // Chain of x509 certs, PEM-encoded
}

/// Indicates how the key was provisioned
#[derive(Debug, Clone, PartialEq)]
pub enum KeySource {
    Tappd,   // Key was provisioned via Tappd
    Random,  // Key was generated locally
}

impl KeyMaterial {
    /// Generate a new key locally using randomness
    pub fn new_random() -> Self {
        let signing_key = SigningKey::random(&mut OsRng);
        Self {
            signing_key,
            source: KeySource::Random,
            certificate_chain: None,
        }
    }

    /// Create KeyMaterial from a response returned by Tappd
    pub fn from_get_key_response(response: &GetKeyResponse) -> Result<Self, String> {
        let signing_key = match SigningKey::from_pkcs8_pem(&response.key) {
            Ok(key) => key,
            Err(e) => {
                eprintln!("Failed to create signing key from Tappd key: {}", e);
                return Ok(KeyMaterial::new_random());
            }
        };
        Ok(Self {
            signing_key,
            source: KeySource::Tappd,
            certificate_chain: Some(response.certificate_chain.clone()),
        })
    }

    /// Returns the raw public key bytes in uncompressed format (04 || X || Y)
    pub fn public_key_bytes(&self) -> Vec<u8> {
        self.signing_key
            .verifying_key()
            .to_encoded_point(false)
            .as_bytes()
            .to_vec()
    }

    /// Returns hex-encoded public key
    pub fn encode_verify_key(&self) -> String {
        let pub_key = self.public_key_bytes();
        hex::encode(pub_key)
    }

    /// Returns the verifying key corresponding to the signing key
    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key().clone()
    }

    /// Constructs verifying key from a hex-encoded public key string
    pub fn verifying_key_from_hex_encoded(
        &self,
        hex_encoded: &str,
    ) -> Result<VerifyingKey, String> {
        let bytes = hex::decode(hex_encoded).map_err(|e| e.to_string())?;
        let point = EncodedPoint::from_bytes(&bytes).map_err(|e| e.to_string())?;
        VerifyingKey::from_encoded_point(&point).map_err(|e| e.to_string())
    }

    /// Computes a report hash (SHA-512) of the public key to embed in attestation
    pub fn report_data_from_key(&self) -> String {
        let pub_key = self.public_key_bytes();
        let hash = Sha512::digest(&pub_key);
        format!("0x{}", hex::encode(hash))
    }

    /// Signs the given message with the private key
    pub fn sign_message(&self, message: &[u8]) -> Signature {
        println!("[sign_message] Signing message with key source: {:?}", self.source);
        self.signing_key.sign(message)
    }
}


#[derive(Serialize, Deserialize)]
pub struct ScoreGernerationInput {
    #[serde(
        serialize_with = "serialize_signatures",
        deserialize_with = "deserialize_signatures"
    )]
    pub all_signatures: Vec<[u8; 65]>, //from client
    #[serde(
        serialize_with = "serialize_nullifiers",
        deserialize_with = "deserialize_nullifiers"
    )]
    pub all_nullifiers: Vec<[u8; 32]>, //from client
    pub owned_accounts_addresses: Vec<Address>, //from client

    pub contract_address: Address, //from client
    pub user_address: Address, //not sure
    pub message: String, // hardcoded ? 
    pub api_url: String, // from tlsn proof
    pub trusted_state_root: H256, // from tlsn ? 
    pub tradify_credit_score: u16, // from client? 
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScoreGernerationOutput {
    #[serde(
        serialize_with = "serialize_nullifiers",
        deserialize_with = "deserialize_nullifiers"
    )]
    pub all_nullifiers: Vec<[u8; 32]>,
    pub contract_address: Address,
    pub user_address: Address,
    pub total_eth_balance: U256,
    pub score: u16,
}


#[derive(Serialize, Deserialize)]
pub struct ScoreGenerationRequest{
    #[serde(
        serialize_with = "serialize_signatures",
        deserialize_with = "deserialize_signatures"
    )]
    pub all_signatures: Vec<[u8; 65]>, //from client
    #[serde(
        serialize_with = "serialize_nullifiers",
        deserialize_with = "deserialize_nullifiers"
    )]
    pub all_nullifiers: Vec<[u8; 32]>, //from client
    pub owned_accounts_addresses: Vec<Address>, //from client
    pub contract_address: Address, //from client
    pub user_address: Address, //not sure
    pub message: String, // hardcoded ? 
    pub tradfi_tlsn_proof : String,
    pub state_root_tlsn_proof : String,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalData {
    pub score: u64,
    pub server_name: String,
    pub state_root_provider: String,
    pub block_number: u64,
    pub tradfi_nullifier: [u8; 32],
    pub tradfi_date_timestamp: u64,
    pub user_address: Address,
    pub all_nullifiers: Vec<[u8; 32]>,
}
