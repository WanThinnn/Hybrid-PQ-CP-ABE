//! Signing key abstraction and software-based backends.

// Re-export shared crypto from tsp-ltv
#[cfg(feature = "legacy-crypto")]
pub use tsp_ltv::crypto::algorithm;
#[cfg(not(feature = "legacy-crypto"))]
pub mod algorithm;
#[cfg(feature = "legacy-crypto")]
pub use tsp_ltv::crypto::verify;

// Local modules (private key operations, signer traits)
#[cfg(feature = "software-signing")]
pub mod software;
pub mod traits;
