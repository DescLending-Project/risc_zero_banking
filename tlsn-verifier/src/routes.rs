use actix_web::{get, post, HttpResponse, Responder};
use serde_json;
use tlsn_core::presentation;
use crate::attestation::{get_attestation_report_with_signature, read_attestation_report};
use crate::{key_manager, score_generator};
use crate::utils::prepare_report_data;
use crate::verifier::{extract_block_number, extract_state_root, extract_tradfi_score, verify_tlsn_proof};
use crate::types::{AttestationError, JournalData, PresentationJSON, ScoreGenerationRequest, ScoreGernerationInput, VerificationError, VerificationResponse};

/// Health check endpoint for readiness/liveness probes
#[get("/health")]
pub async fn health_check() -> impl Responder {
    HttpResponse::Ok().body("OK") // Always returns 200 OK with simple body
}

/// Main verification endpoint that handles TLSN proof verification + attestation
#[post("/verify-tradfi-score")]
pub async fn verify_proof_route(body: String) -> impl Responder {
    println!("[verify_proof_route] Starting verification route handler");

    // Early JSON validation to short-circuit on bad input
    let presentation_json = match PresentationJSON::from_json_str(&body) {
        Ok(p) => p,
        Err(e) => {
            let response = VerificationResponse {
                verification: Err(VerificationError { message: format!("Invalid JSON format: {}", e) }),
                attestation: Err(AttestationError { message: "Attestation skipped due to invalid input".to_string() }),
            };
            return HttpResponse::BadRequest().json(response);
        }
    };

    // Proceed with proof verification
    let verification_result = match verify_tlsn_proof(&presentation_json) {
        Ok(result) => result,
        Err(e) => {
            let response = VerificationResponse {
                verification: Err(e),
                attestation: Err(AttestationError { message: "Attestation skipped due to verification failure".to_string() }),
            };
            return HttpResponse::BadRequest().json(response);
        }
    };

    let credit_score = match extract_tradfi_score(&verification_result){
        Ok(score) => score,
        Err(e) => {
            let response = VerificationResponse {
                verification: Err(VerificationError { message: format!("Failed to extract credit score: {}", e) }),
                attestation: Err(AttestationError { message: "Attestation skipped due to credit score extraction failure".to_string() }),
            };
            return HttpResponse::BadRequest().json(response);
        }
    };

    let key = match key_manager::try_get_key_material() {
        Some(k) => k,
        None => {
            let response = VerificationResponse {
                verification: Err(VerificationError { message: "Key material not initialized".to_string() }),
                attestation: Err(AttestationError { message: "Attestation skipped due to missing key material".to_string() }),
            };
            return HttpResponse::InternalServerError().json(response);
        }
    };
    let report_data = prepare_report_data(credit_score.to_string());
    // Generate an attestation quote with signature and key info
    let attestation = get_attestation_report_with_signature(&report_data).await;
    println!("[verify_proof_route] Attestation report generated successfully");
    // Combine both into a structured response object
    let response = match attestation {
        Ok(report) => {
            VerificationRewsponse {
                verification: verification_result,
                attestation: Ok(report),
            }
        }
        Err(e) => {
            VerificationResponse {
                verification: verification_result,
                attestation: Err(e),
            }
        }
    };

    // Determine HTTP response code based on success/failure cases
    match (&response.verification, &response.attestation) {
        (Ok(_), Ok(_)) => HttpResponse::Ok().json(&response),                     // All good
        (Err(_), Ok(_)) => HttpResponse::BadRequest().json(&response),           // Proof invalid
        (_, Err(_)) => HttpResponse::InternalServerError().json(&response),      // Attestation failure
    }
}

/// Standalone attestation endpoint that returns only the attestation data
#[get("/attestation")]
pub async fn attestation_route() -> impl Responder {
    println!("[attestation] Starting attestation route handler");

    // Generate and return attestation report with signature
    let attestation = get_attestation_report_with_signature("").await;
    match attestation {
        Ok(report) => HttpResponse::Ok().json(report),               // Success
        Err(e) => HttpResponse::InternalServerError().json(e),       // Failure
    }
}


#[post("/generate-score")]
pub async fn generate_score_route(
   body: String
) -> impl Responder {
    println!("[generate_score_route] Starting score generation route handler");
    // first convert from body to ScoreGenerationRequest
    let score_generation_request = match serde_json::from_str::<ScoreGenerationRequest>(&body) {
        Ok(req) => req,
        Err(e) => {
            return HttpResponse::BadRequest().json(VerificationError { message: format!("Invalid JSON format: {}", e) });
        }
    };

    let tradfi_tlsn_proof = score_generation_request.tradfi_tlsn_proof;
    let tradfi_tlsn_proof_json = match PresentationJSON::from_json_str(&tradfi_tlsn_proof) {
        Ok(p) => p,
        Err(e) => {
            return HttpResponse::BadRequest().json(VerificationError { message: format!("Invalid TLSN proof JSON: {}", e) });
        }
    };

    let tradfi_verification_result = match verify_tlsn_proof(&tradfi_tlsn_proof_json) {
        Ok(result) => result,
        Err(e) => {
            return HttpResponse::BadRequest().json(VerificationError { message: format!("TLSN proof verification failed: {}", e) });
        }
    };

    let tradfi_server_name = tradfi_verification_result.server_name;
    let tradfi_date_timestamp = tradfi_verification_result.time;

    let tradfi_credit_score = match extract_tradfi_score(&tradfi_tlsn_proof_json) {
        Ok(score) => score,
        Err(e) => {
            return HttpResponse::BadRequest().json(VerificationError { message: format!("Failed to extract credit score: {}", e) });
        }
    };

    let state_root_tlsn_proof = score_generation_request.state_root_tlsn_proof;
    let state_root_tlsn_proof_json = match PresentationJSON::from_json_str(&state_root_tlsn_proof) {
        Ok(p) => p,
        Err(e) => {
            return HttpResponse::BadRequest().json(VerificationError { message: format!("Invalid state root TLSN proof JSON: {}", e) });
        }
    };

    let state_root_verification_result = match verify_tlsn_proof(&state_root_tlsn_proof_json) {
        Ok(result) => result,
        Err(e) => {
            return HttpResponse::BadRequest().json(VerificationError { message: format!("State root TLSN proof verification failed: {}", e) });
        }
    };

    let state_root_provider = state_root_verification_result.server_name;
    let block_number = match extract_block_number(verification_result) {
        Ok(number) => number,
        Err(e) => {
            return HttpResponse::BadRequest().json(VerificationError { message: format!("Failed to extract block number: {}", e) });
        }
    };

    let trusted_state_root = match extract_state_root(verification_result) {
        Ok(root) => root,
        Err(e) => {
            return HttpResponse::BadRequest().json(VerificationError { message: format!("Failed to extract state root: {}", e) });
        }
    };

    let score_generation_input = ScoreGenerationInput {
        all_signatures: score_generation_request.all_signatures,
        all_nullifiers: score_generation_request.all_nullifiers,
        owned_accounts_addresses: score_generation_request.owned_accounts_addresses,
        contract_address: score_generation_request.contract_address,
        user_address: score_generation_request.user_address,
        message: score_generation_request.message,
        api_url: state_root_provider,
        trusted_state_root: trusted_state_root,
        tradify_credit_score: tradfi_credit_score,
    };

    let score_generator_output = match score_generator::generate_score(score_generation_input).await {
        Ok(result) => result,
        Err(e) => {
            return HttpResponse::InternalServerError().json(VerificationError { message: format!("Score generation failed: {}", e) });
        }
    };

    let journal_data = JournalData{
        score : score_generator_output.score as u64, // Convert to u64 for consistency
        server_name: tradfi_server_name,
        state_root_provider : state_root_provider,
        block_number: block_number,
        tradfi_nullifier : None, // Placeholder for tradfi nullifier, if needed
        tradfi_date_timestamp : tradfi_date_timestamp,
        user_address: score_generator_output.user_address,
        all_nullifiers : score_generation_request.all_nullifiers,
    };

    // convert the journal data to JSON and to string
    let journal_data_json = match serde_json::to_string(&journal_data) {
        Ok(json) => json,
        Err(e) => {
            return HttpResponse::InternalServerError().json(VerificationError { message: format!("Failed to serialize journal data: {}", e) });
        }
    };

    // call the prepare report data function
    let report_data = prepare_report_data(&journal_data_json);

    let signed_attestation = match get_attestation_report_with_signature(&report_data).await {
        Ok(report) => report,
        Err(e) => {
            return HttpResponse::InternalServerError().json(VerificationError { message: format!("Failed to get attestation report: {}", e) });
        }
    };

    // Create the final response object
    let response = VerificationResponse {
        verification: Ok(verification_result),
        attestation: Ok(signed_attestation),
    };
    
}
