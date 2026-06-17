//! NIST Post-Quantum Cryptography implementation for Hanzo Node
//!
//! Implements FIPS 203 (ML-KEM), FIPS 204 (ML-DSA), and FIPS 205 (SLH-DSA)
//! with support for hybrid modes and privacy tiers.

pub mod attestation;
pub mod config;
pub mod errors;
pub mod hybrid;
pub mod kdf;
pub mod kem;
pub mod privacy_tiers;
pub mod signature;
pub mod wire_protocol;

pub use config::PqcConfig;
pub use errors::{PqcError, Result};
pub use hybrid::{HybridKem, HybridMode};
pub use kdf::{Kdf, KdfAlgorithm};
pub use kem::{DecapsulationKey, EncapsulationKey, Kem, KemAlgorithm, KemKeyPair};
pub use privacy_tiers::{CapabilityMatrix, PrivacyTier, RuntimeRequirements};
pub use signature::{Signature, SignatureAlgorithm, SigningKey, VerifyingKey};

// Re-export saorsa-pqc for post-quantum crypto
pub use saorsa_pqc;

/// Initialize the PQC subsystem with FIPS-compliant RNG
pub fn init() -> Result<()> {
    // Ensure we're using a FIPS-compliant RNG
    #[cfg(feature = "fips-mode")]
    {
        verify_fips_rng()?;
    }

    Ok(())
}

#[cfg(feature = "fips-mode")]
fn verify_fips_rng() -> Result<()> {
    // Verify SP 800-90A compliant RNG.
    // getrandom 0.3+ renamed the free `getrandom(buf)` fn to `fill(buf)`.
    let mut buf = [0u8; 32];
    getrandom::fill(&mut buf).map_err(|e| PqcError::RngError(e.to_string()))?;
    Ok(())
}
