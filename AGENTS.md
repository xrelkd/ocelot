# AGENTS.md

This file provides guidance for agentic coding assistants working in the Ocelot repository.

## Build, Lint, and Test Commands

### Standard Commands (via Nix)

The project uses Nix for reproducible development environments.

```bash
# Build all targets
cargo build              # Debug build
cargo build --release    # Release build

# Build single crate
cargo build -p ocelot-entry
cargo build -p ocelot-supervise

# Linting and formatting
cargo fmt --all --check       # Check formatting (rust)
cargo fmt --all               # Format Rust code
cargo clippy-all              # Run clippy on all targets
treefmt                       # Format all file types

# Testing
cargo nextest-all             # Run all unit tests with retries
cargo nextest run             # Same as above
cargo test-all                # Run tests with cargo test
cargo test <test_name>        # Run single test by name
cargo test --test <file_name> # Run single integration test file

# Integration test script
cargo build --release && ./tests/test-entry.sh
```

### Running Without Nix

```bash
# Requires Rust toolchain (rust-toolchain.toml specifies stable with clippy, rustfmt)
cargo build
cargo clippy --all-targets --all-features
cargo nextest run  # Requires cargo-nextest installed
```

## Code Style Guidelines

### Rust Edition and Formatting

- **Edition**: 2024
- **Formatter**: rustfmt with configuration in `rustfmt.toml`
- **Max width**: 100 columns
- **Imports**: Granularity "Crate", grouped as StdExternalCrate, layout "Mixed"
- **Use field init shorthand**: true
- **Trailing comma**: Vertical

### Naming Conventions

- **Modules, functions, variables**: `snake_case`
- **Types (structs, enums, traits)**: `CamelCase`
- **Constants**: `SCREAMING_SNAKE_CASE`
- **Test modules/functions**: `snake_case` with `#[cfg(test)]`

### Error Handling

- **Library**: `snafu` (version 0.9) is the standard for error handling
- **Pattern**: Derive `Snafu` on error enums; use `.context()` for adding context
- **Unwrap/expect**: Avoid in library code; only acceptable in tests or main
- **Result types**: Return `Result<T, Error>` where `Error` is a snafu-derived enum
- **Example**:

  ```rust
  #[derive(Debug, Snafu)]
  #[snafu(visibility(pub))]
  pub enum Error {
      InvalidInput { input: String, source: std::ffi::NulError },
      #[snafu(display("Failed to create signal handler, error: {source}"))]
      CreateSignalHandler { source: std::io::Error },
  }
  ```

### Imports

- Group imports according to rustfmt: `use std::...;` then `use external::...;` then `use crate::...;`
- Prefer `use` statements over fully qualified paths in code
- Re-export public API with `pub use` in `lib.rs`

### Types and Interfaces

- **Prefer** generic parameters with trait bounds: `where` clauses for complex bounds
- **Avoid** `unwrap()` and `expect()` in production code; use proper error propagation
- **Async**: Uses tokio with `rt-multi-thread` runtime; signal handling uses `signal-hook`
- **FFI**: When using `fork()` or other unsafe, add `#[allow(unsafe_code)]` and SAFETY comment

### Linting

- **Workspace lints**: Strict; nearly all lints set to `deny` for both Rust and Clippy
- **Clippy**: `all = "deny"` plus `cargo = "deny"`, `nursery = "deny"`, `pedantic = "deny"`
- **Allowed**: `module_name_repetitions`, `multiple_crate_versions` (priority 1)
- **Async functions in traits**: `allow` (workspace-level)

### Testing

- **Unit tests**: Inline with `#[cfg(test)]` modules or separate `tests/` directory
- **Integration tests**: In `tests/` directory as standalone executables
- **Nextest**: Preferred test runner; configured with `NEXTEST_RETRIES=5`
- **Integration test script**: `tests/test-entry.sh` requires `sudo` for `unshare`

### Documentation

- **Rustdoc**: Use `///` for public items; generate with `cargo doc-all`
- **Examples**: Include code examples in doc comments where helpful
- **CLI**: Uses clap derive with rich doc comments for help text

### Safety and Unsafe Code

- **Unsafe**: Minimize usage; only when absolutely necessary (e.g., `fork`)
- **Pattern**: All `unsafe` blocks must have a SAFETY comment explaining why it's sound
- **Zu**: The project includes `#![deny(unsafe_code)]` at workspace level; exceptions require explicit `#[allow(unsafe_code)]`

### Project Structure

- **Workspace**: Root Cargo.toml manages multiple crates in `crates/`
- **Binary**: `ocelot/` crate produces the main binary
- **Library crates**:
  - `crates/entry` - process supervisor
  - `crates/idle` - minimalist PID 1 for holding namespaces
  - `crates/supervise` - advanced supervisor with reaper/supervisor workers
  - `crates/zombie` - zombie process generator for testing
- **Versioning**: Workspace-managed; uses `shadow-rs` for build-time version info

### Commit Messages

- Uses `commitlint` for validation
- Follows Conventional Commits format
- See `.github/commitlint.config.mjs` (if modifying rules)

### Additional Tools

- **Formatting**: Also uses `treefmt` to format non-Rust files: prettier (JSON/MD/JS/TS), taplo (TOML), nixfmt (Nix), shfmt + shellcheck (Shell), hclfmt (HCL)
- **Static analysis**: Codespell for typo detection

## Not Found

- No `.cursorrules` or `.cursor/rules/` files present.
- No `.github/copilot-instructions.md` present.

## Quick Reference for Common Tasks

```bash
# Build and test after changes
cargo nextest run

# Check formatting and linting
cargo fmt --all --check
treefmt
cargo clippy-all

# Run a specific unit test
cargo test test_name

# Build release binary for integration testing
cargo build --release && ./tests/test-entry.sh

# Generate shell completions
cargo run -- completions zsh > /etc/zsh/completions/_ocelot
```
