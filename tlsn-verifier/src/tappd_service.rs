
use crate::types::TappdError;
use hyper::{Body, Client, Request};
use hyperlocal::{UnixClientExt, Uri};
use serde_json::json;
use hyper::Response;


pub async fn send_quote_request(
    custom_evidence : &str,
) -> Result<Response<Body>, TappdError> {
    println!("[send_quote_request] Sending quote request to Tappd service");
    let client = Client::unix();
    let uri: hyperlocal::Uri = Uri::new("/var/run/tappd.sock", "/prpc/Tappd.TdxQuote?json").into();
    // Build HTTP POST request with JSON body
    let req = Request::post(uri)
        .header("Content-Type", "application/json")
        .body(Body::from(custom_evidence.to_string()))
        .map_err(|e| {
            TappdError {
                message: format!("Failed to build request: {}", e),
            }
        })?;
    println!("[send_quote_request] Request built successfully: {:?}", req);
    // Send the request to the tappd socket and await response
    let res = client.request(req).await.map_err(|e| {
        TappdError {
            message: format!("Failed to send request: {}", e),
        }
    })?;
    println!("[send_quote_request] Response received from Tappd service: {:?}", res);
    Ok(res)
}


pub async fn send_key_request() -> Result<Response<Body>, TappdError> {
    println!("[send_key_request] Requesting key material from Tappd service");
    let client = Client::unix();
    let uri: hyperlocal::Uri = Uri::new("/var/run/tappd.sock", "/prpc/Tappd.DeriveKey?json").into();

    // Build HTTP POST request with empty JSON body
    let req = Request::post(uri)
        .header("Content-Type", "application/json")
        .body(Body::from(json!({}).to_string()))
        .map_err(|e| {
            TappdError {
                message: format!("Failed to build request: {}", e),
            }
        })?;
    println!("[send_key_request] Request built successfully: {:?}", req);
    // Send the request to the tappd socket and await response
    let res = client.request(req).await.map_err(|e| {
        TappdError {
            message: format!("Failed to send request: {}", e),
        }
    })?;
    println!("[send_key_request] Response received from Tappd service: {:?}", res);
    Ok(res)
}