//! Hanzo KBS (Key Broker Service) Library
//!
//! Provides Key Management Service (KMS) and Key Broker Service (KBS) functionality
//! for confidential computing and privacy-preserving agent execution in Hanzo nodes.
//!
//! This crate implements the KMS/KBS split architecture where:
//! - KMS handles key lifecycle management and storage
//! - KBS handles attestation verification and policy-based key release

// PQC vault scaffolding has fields/methods reserved for the next milestone;
// base64 deprecations come from a transitive dep — neither is actionable here.
#![allow(dead_code, deprecated)]

pub mod attestation;
pub mod error;
pub mod kbs;
pub mod kms;
pub mod types;
pub mod vault;

#[cfg(feature = "pqc")]
pub mod pqc_integration;

#[cfg(feature = "pqc")]
pub mod pqc_vault;

pub use error::{Result, SecurityError};
pub use kbs::KeyBrokerService;
pub use kms::KeyManagementService;
pub use types::*;

// Re-export submodules from kms
pub use kms::{api, memory_kms};
