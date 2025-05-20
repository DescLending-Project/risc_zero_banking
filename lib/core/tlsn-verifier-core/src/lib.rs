pub mod types;
pub mod verification;

// Expose the main verification function with consistent signature
// regardless of which implementation is used
pub use verification::verify_tlsn_presentation;
