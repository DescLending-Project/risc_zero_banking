use std::env;
use dotenvy::dotenv;

/// Loads environment variables from a `.env` file (if present).
/// Useful for local development and testing without setting env vars globally.
pub fn load_env() {
    dotenv().ok();
}

/// Retrieves the API key from the environment.
/// Panics if `TLSN_VERIFIER_API_KEY` is not set.
pub fn get_api_key() -> String {
    env::var("TLSN_VERIFIER_API_KEY").expect("API_KEY must be set")
}

/// Returns the host to bind the verifier server to.
/// Defaults to `127.0.0.1` if `TLSN_VERIFIER_HOST` is not set.
pub fn get_host() -> String {
    env::var("TLSN_VERIFIER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string())
}

/// Returns the port to bind the verifier server to.
/// Defaults to `8080` if `TLSN_VERIFIER_PORT` is not set.
/// Panics if the value is not a valid number.
pub fn get_port() -> u16 {
    env::var("TLSN_VERIFIER_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .expect("PORT must be a number")
}

/// Retrieves a list of accepted server names for TLS verification.
/// Expects a comma-separated string in `TLSN_VERIFIER_ACCEPTED_SERVER_NAMES`.
pub fn get_server_names() -> Vec<String> {
    env::var("TLSN_VERIFIER_ACCEPTED_SERVER_NAMES")
        .unwrap_or_default()
        .split(',')                         // Split the string by commas
        .map(|s| s.trim().to_string())      // Trim and convert to String
        .filter(|s| !s.is_empty())          // Remove empty entries
        .collect()                          // Collect into a Vec<String>
}

/// Retrieves the accepted TLSN core version to verify against.
/// Defaults to `0.1.0-alpha.10` if not set.
pub fn get_tlsn_core_version() -> String {
    env::var("TLSN_VERIFIER_ACCEPTED_VERSION").unwrap_or_else(|_| "0.1.0-alpha.10".to_string())
}
