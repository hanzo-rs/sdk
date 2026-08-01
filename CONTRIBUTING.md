# Contributing to Hanzo Rust SDK

First off, thank you for considering contributing to Hanzo Rust SDK! It's people like you that make this project great.

## Code of Conduct

This project and everyone participating in it is governed by our [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code.

## How Can I Contribute?

### Reporting Bugs

Before creating bug reports, please check existing issues as you might find out that you don't need to create one. When you are creating a bug report, please include as many details as possible using our [bug report template](.github/ISSUE_TEMPLATE/bug_report.md).

### Suggesting Enhancements

Enhancement suggestions are tracked as GitHub issues. When creating an enhancement suggestion, please use our [feature request template](.github/ISSUE_TEMPLATE/feature_request.md) and include:

* A clear and descriptive title
* A detailed description of the proposed enhancement
* Examples of how the enhancement would be used
* Why this enhancement would be useful

### Pull Requests

Please follow these steps:

1. Fork the repo and create your branch from `main`
2. If you've added code that should be tested, add tests
3. If you've changed APIs, update the documentation
4. Ensure the test suite passes (`cargo test --all`)
5. Make sure your code lints (`cargo clippy --all -- -D warnings`)
6. Format your code (`cargo fmt --all`)
7. Create a Pull Request using our [PR template](.github/pull_request_template.md)

## Development Setup

### Prerequisites

- Rust 1.70.0 or later
- Git

### Getting Started

1. Clone your fork:
```bash
git clone https://github.com/YOUR_USERNAME/rust-sdk.git
cd rust-sdk
```

2. Add upstream remote:
```bash
git remote add upstream https://github.com/hanzoai/rust-sdk.git
```

3. Create a new branch:
```bash
git checkout -b feature/your-feature-name
```

4. Make your changes and commit:
```bash
git add .
git commit -m "feat: add amazing feature"
```

### Running Tests

```bash
# Run all tests
cargo test --all

# Run tests with all features
cargo test --all --all-features

# Run specific crate tests
cargo test -p hanzo-kbs

# Run with coverage (requires cargo-tarpaulin)
cargo tarpaulin --all --out Html
```

### Code Style

We use `rustfmt` for code formatting and `clippy` for linting:

```bash
# Format code
cargo fmt --all

# Check formatting
cargo fmt --all -- --check

# Run clippy
cargo clippy --all --all-features -- -D warnings
```

### Documentation

- Document all public APIs
- Include examples in doc comments
- Run `cargo doc --all --no-deps --open` to preview documentation

### Commit Messages

We follow [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` New feature
- `fix:` Bug fix
- `docs:` Documentation only changes
- `style:` Code style changes (formatting, etc)
- `refactor:` Code change that neither fixes a bug nor adds a feature
- `perf:` Performance improvement
- `test:` Adding or updating tests
- `chore:` Changes to build process or auxiliary tools

Examples:
```
feat: add GPU TEE-I/O support for Blackwell
fix: resolve race condition in key rotation
docs: update KBS API documentation
perf: optimize PQC key generation
```

### Testing Guidelines

- Write unit tests for new functionality
- Aim for >80% code coverage
- Test error conditions and edge cases
- Use property-based testing where appropriate
- Mock external dependencies

Example test:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_key_generation() {
        let kbs = Kbs::new();
        let key = kbs.generate_key().await.unwrap();
        assert_eq!(key.len(), 32);
    }
}
```

### Security

- Never commit secrets or credentials
- Use secure random number generation
- Validate all inputs
- Follow OWASP secure coding practices
- Report security vulnerabilities to security@hanzo.ai

### Performance

- Benchmark performance-critical code
- Use `cargo bench` for benchmarking
- Profile before optimizing
- Document performance characteristics

### Release Process

One crate per tag, `<crate>-v<semver>`.

1. Bump that crate's `version` in its `Cargo.toml` — a patch bump, `x.y.Z+1`
2. Merge it
3. `git tag <crate>-v<semver> && git push origin <crate>-v<semver>`

`.github/workflows/release.yml` refuses the tag if the manifest says a different
version, packs, publishes, and then proves the version against the public
crates.io API. Re-pushing an already-published tag is a no-op when the digest
matches and a hard failure when it does not.

## Community

- Join our [Discord](https://discord.gg/hanzoai)
- Follow us on [Twitter](https://twitter.com/hanzoai)
- Read our [blog](https://blog.hanzo.ai)

## Recognition

Contributors will be recognized in our [CONTRIBUTORS.md](CONTRIBUTORS.md) file.

Thank you for contributing to Hanzo Rust SDK! 🎉