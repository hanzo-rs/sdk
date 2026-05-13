//! Vault implementations for software-managed privacy tiers (Tier 0 / 1).
//!
//! GPU-backed vaults live in [`crate::pqc_vault`] under the `pqc` feature —
//! that is the canonical place for hardware-attested tiers.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::error::{Result, SecurityError};
use crate::types::{PrivacyTier, KeyId};
use crate::kms::KeyManagementService;

/// Trait for secure key storage and usage
#[async_trait]
pub trait KeyVault: Send + Sync {
    /// Get the privacy tier this vault supports
    fn tier(&self) -> PrivacyTier;
    
    /// Store a key in the vault
    async fn store_key(&self, key_id: &KeyId, key_data: &[u8]) -> Result<()>;
    
    /// Use a key for an operation (key never leaves vault in plaintext)
    async fn use_key<F, R>(&self, key_id: &KeyId, operation: F) -> Result<R>
    where
        F: FnOnce(&[u8]) -> R + Send,
        R: Send;
    
    /// Delete a key from the vault
    async fn delete_key(&self, key_id: &KeyId) -> Result<()>;
    
    /// Check if vault is properly initialized
    async fn is_initialized(&self) -> Result<bool>;
}

/// Configuration for different vault types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    pub vault_type: VaultType,
    pub kms_url: Option<String>,
    pub kbs_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum VaultType {
    File { path: String },
    Sim { eid: String },
    GpuCc { device_id: u32 },
    GpuTeeIo { device_id: u32, mig_instance: Option<u32> },
}

/// Tier 1: File-based vault with at-rest encryption
pub struct FileVault<K: KeyManagementService> {
    kms: K,
    base_path: std::path::PathBuf,
    tenant_id: String,
}

impl<K: KeyManagementService> FileVault<K> {
    pub fn new(kms: K, base_path: impl AsRef<Path>, tenant_id: String) -> Self {
        Self {
            kms,
            base_path: base_path.as_ref().to_path_buf(),
            tenant_id,
        }
    }
}

#[async_trait]
impl<K: KeyManagementService> KeyVault for FileVault<K> {
    fn tier(&self) -> PrivacyTier {
        PrivacyTier::AtRest
    }
    
    async fn store_key(&self, key_id: &KeyId, key_data: &[u8]) -> Result<()> {
        // Wrap key with tenant KEK before storing
        let wrapped = self.kms.wrap_key(key_data, key_id).await?;
        
        let key_path = self.base_path.join(format!("{}.key", key_id.0));
        tokio::fs::create_dir_all(&self.base_path).await?;
        tokio::fs::write(&key_path, &wrapped).await?;
        
        Ok(())
    }
    
    async fn use_key<F, R>(&self, key_id: &KeyId, operation: F) -> Result<R>
    where
        F: FnOnce(&[u8]) -> R + Send,
        R: Send,
    {
        let key_path = self.base_path.join(format!("{}.key", key_id.0));
        let wrapped = tokio::fs::read(&key_path).await?;
        
        // Unwrap key for use
        let key_data = self.kms.unwrap_key(&wrapped, key_id).await?;
        
        // Use key and zeroize after
        let result = operation(&key_data);
        
        // Zeroize key data
        drop(key_data); // In production, use zeroize crate
        
        Ok(result)
    }
    
    async fn delete_key(&self, key_id: &KeyId) -> Result<()> {
        let key_path = self.base_path.join(format!("{}.key", key_id.0));
        tokio::fs::remove_file(&key_path).await?;
        Ok(())
    }
    
    async fn is_initialized(&self) -> Result<bool> {
        Ok(self.base_path.exists())
    }
}

/// Tier 1+: SIM-based vault with hardware-bound keys
pub struct SimVault<K: KeyManagementService> {
    kms: K,
    sim_eid: String,
    file_vault: FileVault<K>, // Fallback storage
}

impl<K: KeyManagementService + Clone> SimVault<K> {
    pub fn new(kms: K, base_path: impl AsRef<Path>, tenant_id: String, sim_eid: String) -> Self {
        Self {
            kms: kms.clone(),
            sim_eid,
            file_vault: FileVault::new(kms, base_path, tenant_id),
        }
    }
    
    async fn bind_to_sim(&self, key_data: &[u8]) -> Result<Vec<u8>> {
        // TODO: Implement SIM binding using EID
        // For now, just add EID to key derivation
        let mut hasher = blake3::Hasher::new();
        hasher.update(key_data);
        hasher.update(self.sim_eid.as_bytes());
        Ok(hasher.finalize().as_bytes().to_vec())
    }
}

#[async_trait]
impl<K: KeyManagementService + Clone> KeyVault for SimVault<K> {
    fn tier(&self) -> PrivacyTier {
        PrivacyTier::AtRest // Still Tier 1, but with SIM binding
    }
    
    async fn store_key(&self, key_id: &KeyId, key_data: &[u8]) -> Result<()> {
        let sim_bound = self.bind_to_sim(key_data).await?;
        self.file_vault.store_key(key_id, &sim_bound).await
    }
    
    async fn use_key<F, R>(&self, key_id: &KeyId, operation: F) -> Result<R>
    where
        F: FnOnce(&[u8]) -> R + Send,
        R: Send,
    {
        self.file_vault.use_key(key_id, operation).await
    }
    
    async fn delete_key(&self, key_id: &KeyId) -> Result<()> {
        self.file_vault.delete_key(key_id).await
    }
    
    async fn is_initialized(&self) -> Result<bool> {
        // TODO: Check SIM availability
        self.file_vault.is_initialized().await
    }
}
