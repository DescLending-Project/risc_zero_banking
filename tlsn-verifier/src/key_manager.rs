use crate::types::*;
use crate::types::KeyManagerError;
use once_cell::sync::OnceCell;
use crate::tappd_service;
/// Holds a private ECDSA signing key, its origin, and optional certificate chain

/// Singleton that stores the initialized KeyMaterial
static KEY_MATERIAL: OnceCell<KeyMaterial> = OnceCell::new();


async fn derive_key_from_tappd() -> Result<GetKeyResponse, KeyManagerError> {
    println!("[derive_key_from_tappd] Requesting key material from Tappd service");
    let res = tappd_service::send_key_request().await.map_err(|e| {
        KeyManagerError {
            message: format!("Tappd Service Error: {}", e.message),
        }
    })?;
    println!("[derive_key_from_tappd] Response received from Tappd service");
    let body_bytes = hyper::body::to_bytes(res.into_body())
        .await
        .map_err(|e| KeyManagerError {
            message: format!("Failed to read response body: {}", e),
        })?;
    println!("[derive_key_from_tappd] Response body read successfully");
    let parsed: GetKeyResponse =
        serde_json::from_slice(&body_bytes).map_err(|e| KeyManagerError {
            message: format!("Failed to parse GetKeyResponse: {}", e),
        })?;
    println!("[derive_key_from_tappd] GetKeyResponse parsed successfully");
    Ok(parsed)
}


pub async fn init_key_material_from_tappd_socket() -> Result<(), KeyManagerError> {
    let key_material = match derive_key_from_tappd().await {
        Ok(key_response) => {
            // Try to parse key and certificate from response
            println!("Successfully derived key from Tappd");
            match KeyMaterial::from_get_key_response(&key_response) {
                Ok(km) => {
                    println!("Successfully created signing key from Tappd key");
                    km
                }
                Err(e) => {
                    println!("Error creating signing key from Tappd key: {}", e);
                    println!("Falling back to random key generation");
                    KeyMaterial::new_random()
                }
            }
        }
        Err(e) => {
            // If Tappd fails, generate a local key instead
            println!("Error deriving key from Tappd: {:?}", e);
            println!("Falling back to random key generation");
            KeyMaterial::new_random()
        }
    };


    // Set the global KEY_MATERIAL (only once)
    KEY_MATERIAL
        .set(key_material)
        .map_err(|_| KeyManagerError {
            message: "Key material already initialized".to_string(),
        })?;

    Ok(())
}

/// Safe getter: returns `Some(&KeyMaterial)` if already initialized
pub fn try_get_key_material() -> Option<&'static KeyMaterial> {
    KEY_MATERIAL.get()
}
