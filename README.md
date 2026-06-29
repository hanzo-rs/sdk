<p align="center"><img src=".github/hero.svg" alt="rust-sdk" width="880"></p>

# Hanzo Rust SDK

[![CI](https://github.com/hanzoai/rust-sdk/actions/workflows/ci.yml/badge.svg)](https://github.com/hanzoai/rust-sdk/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

Official Rust SDK for Hanzo AI infrastructure, providing secure key management, post-quantum cryptography, LLM safety, content extraction, and core primitives for building AI applications.

## Crates

This workspace contains the following crates:

### AI Safety & Content

| Crate | Description |
|-------|-------------|
| [`hanzo-guard`](crates/hanzo-guard) | LLM I/O sanitization with PII redaction, injection detection, rate limiting |
| [`hanzo-extract`](crates/hanzo-extract) | Content extraction from web/PDF with built-in sanitization |

### Security & Cryptography

| Crate | Description |
|-------|-------------|
| [`hanzo-kbs`](crates/hanzo-kbs) | Key Broker Service for confidential computing with privacy tiers |
| [`hanzo-pqc`](crates/hanzo-pqc) | Post-Quantum Cryptography (ML-KEM, ML-DSA, hybrid modes) |

### Core Infrastructure

| Crate | Description |
|-------|-------------|
| [`hanzo-message-primitives`](crates/hanzo-message-primitives) | Core message types and schemas for Hanzo AI systems |

## Getting Started

Add the desired crates to your `Cargo.toml`:

```toml
[dependencies]
hanzo-guard = "0.1"
hanzo-extract = "0.1"
hanzo-kbs = "0.1"
hanzo-pqc = "0.1"
```

## Examples

### LLM Input Sanitization

```rust
use hanzo_guard::{Guard, GuardConfig, SanitizeResult};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let guard = Guard::new(GuardConfig::default());

    let result = guard.sanitize_input("My SSN is 123-45-6789").await?;

    match result {
        SanitizeResult::Clean(text) => println!("Clean: {text}"),
        SanitizeResult::Redacted { text, .. } => println!("Redacted: {text}"),
        SanitizeResult::Blocked { reason, .. } => println!("Blocked: {reason}"),
    }

    Ok(())
}
```

### Content Extraction

```rust
use hanzo_extract::{Extractor, WebExtractor};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let extractor = WebExtractor::default();

    // Extract and sanitize web content
    let result = extractor.extract_sanitized("https://example.com").await?;

    println!("Title: {:?}", result.title);
    println!("Text: {}", result.text);

    Ok(())
}
```

### Key Management with Privacy Tiers

```rust
use hanzo_kbs::{
    kbs::Kbs,
    kms::{Kms, MemoryKms},
    types::PrivacyTier,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kms = MemoryKms::new().await?;
    let kbs = Kbs::new(kms);
    
    // Initialize for GPU Confidential Computing
    kbs.initialize_vault(PrivacyTier::GpuCc, None).await?;
    
    Ok(())
}
```

### Post-Quantum Cryptography

```rust
use hanzo_pqc::{
    kem::{Kem, KemAlgorithm, MlKem},
    signature::{Signature, SignatureAlgorithm, MlDsa},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate ML-KEM keypair
    let ml_kem = MlKem::new();
    let keypair = ml_kem.generate_keypair(KemAlgorithm::MlKem768).await?;
    
    // Generate ML-DSA keypair
    let ml_dsa = MlDsa::new();
    let (verifying_key, signing_key) = ml_dsa.generate_keypair(SignatureAlgorithm::MlDsa65).await?;
    
    Ok(())
}
```

## Development

```bash
# Build all crates
cargo build --all

# Run tests
cargo test --all

# Build for release
cargo build --release --all
```

## Releasing

This monorepo supports per-package releases:

```bash
# Release a single package
./scripts/release-package.sh hanzo-kbs 0.1.1

# Release all packages
./scripts/release-package.sh all 0.2.0
```

See [PUBLISHING.md](PUBLISHING.md) for detailed release instructions.

## License

All crates are dual licensed under MIT OR Apache-2.0.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Links

- [Hanzo AI](https://hanzo.ai)
- [Documentation](https://docs.rs/hanzo-kbs)
- [GitHub](https://github.com/hanzoai/rust-sdk)