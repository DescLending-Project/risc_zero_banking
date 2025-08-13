// Declare internal modules
mod attestation;
mod auth;
mod config;
mod key_manager;
mod routes;
mod types;
mod verifier;
mod tappd_service;
mod utils;
mod score_generator;
use crate::auth::ApiKeyAuth;
use crate::routes::*;
use actix_web::{App, HttpServer};
use std::time::Duration;

/// Main entry point for the TLSN Verifier web server
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load environment variables from `.env` and system environment
    config::load_env();

    // Test outbound connectivity
    println!("Testing outbound connectivity...");
    test_outbound_request().await;

    // Initialize cryptographic key material (preferably from Tappd socket)
    key_manager::init_key_material_from_tappd_socket().await.map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            "Key material initialization failed",
        )
    })?;

    // Read server binding configuration from env
    let host = config::get_host();
    let port = config::get_port();

    // Print server startup information for debugging/logging
    println!("Running on http://{}:{}", host, port);
    println!("Accepted Server Names: {:?}", config::get_server_names());
    println!(
        "Accepted TLSN Core Version: {}",
        config::get_tlsn_core_version()
    );
    println!("Environment variables loaded successfully.");

    // Launch the HTTP server
    HttpServer::new(|| {
        App::new()
            // Apply API key authorization middleware to all routes
            .wrap(ApiKeyAuth)
            // Register health check route
            .service(health_check)
            // Register proof verification endpoint
            .service(verify_proof_route)
            // Register attestation reporting endpoint
            .service(attestation_route)
    })
    .bind((host.as_str(), port))? // Bind to the configured host and port
    .run()
    .await
}

/// Test function to verify outbound network connectivity
async fn test_outbound_request() {
    // Use reqwest for HTTP requests - make sure to add this to your Cargo.toml
    // reqwest = { version = "0.11", features = ["json"] }
    
    match reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build() {
            Ok(client) => {
                // Try accessing JSONPlaceholder (a dummy API service)
                match client.get("https://jsonplaceholder.typicode.com/todos/1").send().await {
                    Ok(response) => {
                        println!("✅ Successfully connected to external API");
                        println!("HTTP Status: {}", response.status());
                        
                        // Try to get the response body
                        match response.text().await {
                            Ok(text) => println!("Response: {}", text),
                            Err(e) => println!("Error reading response: {}", e),
                        }
                    },
                    Err(e) => println!("❌ Failed to connect: {}", e),
                }
                
                // Also try Google as a backup test
                match client.get("https://www.google.com").send().await {
                    Ok(response) => println!("✅ Successfully connected to Google (status: {})", response.status()),
                    Err(e) => println!("❌ Failed to connect to Google: {}", e),
                }
            },
            Err(e) => println!("❌ Failed to create HTTP client: {}", e),
        }
}