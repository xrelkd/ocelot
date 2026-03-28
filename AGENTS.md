# AGENTS.md

This file provides guidance for agentic coding assistants working in the Ocelot repository.

## Build, Lint, and Test Commands

### Using Nix (recommended)

The project uses Nix for reproducible development. The Nix devshell provides custom cargo commands as wrappers that invoke cargo with appropriate arguments.

```bash
# Build targets
cargo build              # Debug build
cargo build --release    # Release build
cargo build -p ocelot    # Build only the main binary

# Formatting and linting
cargo fmt --all --check       # Check Rust code formatting
treefmt                       # Format all file types (Rust, JS, JSON, TOML, Nix, Shell, HCL)
cargo clippy-all              # Run clippy on all targets (wrapper: cargo clippy --workspace --all-targets)
cargo doc-all                 # Generate documentation (wrapper: cargo doc --workspace --no-deps --bins --all-features)
cargo test-all                # Run all tests (including ignored) with cargo test (wrapper)

# Testing
cargo nextest-all             # Run all tests with retries using nextest (preferred)
cargo test                    # Run tests with cargo test
cargo test <test_name>        # Run single test by name
cargo test --test <file_name> # Run single integration test file
```

### Running Without Nix

If not using Nix, install the Rust toolchain (as specified in `rust-toolchain.toml`) and any required cargo plugins (e.g., cargo-nextest).

```bash
cargo build
cargo clippy --workspace --all-targets
cargo test
cargo nextest run --workspace  # Requires cargo-nextest
cargo fmt --all --check
treefmt  # Install separately if needed
cargo doc --workspace --no-deps --bins --all-features
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
- **FFI**: When using `fork()` or other unsafe, add `#[expect(unsafe_code, reason = "...")]` and SAFETY comment

### Linting

- **Workspace lints**: Strict; nearly all lints set to `deny` for both Rust and Clippy
- **Clippy**: `all = "deny"` plus `cargo = "deny"`, `nursery = "deny"`, `pedantic = "deny"`
- **Allowed**: `module_name_repetitions`, `multiple_crate_versions` (priority 1)
- **Async functions in traits**: `allow` (workspace-level)

#### Lint Attribute Suppression

- Use `#[expect(lint_name, reason = "explanation")]` instead of `#[allow(lint_name)]` for all intentional lint suppressions
- The `reason` must explain **why** the suppression is valid for that specific location
- Redundant RATIONALE/SAFETY comment blocks should be removed after conversion
- Workspace-level allowances in `Cargo.toml` are separate and may still use `allow`

Example:

```rust
// Before
#[allow(unsafe_code)]
// SAFETY: fork is safe in single-threaded context.
unsafe { unistd::fork()? }

// After
#[expect(unsafe_code, reason = "Fork is safe in single-threaded context")]
unsafe { unistd::fork()? }
```

### Testing

- **Unit tests**: Inline with `#[cfg(test)]` modules or separate `tests/` directory (within each crate)
- **Integration tests**: In `crates/*/tests/` as standalone test executables
- **Nextest**: Preferred test runner; configured with `NEXTEST_RETRIES=5` (wrapper sets this automatically)
- **Test utilities**: Provided in the `ocelot-test-utils` crate

### Documentation

- **Rustdoc**: Use `///` for public items; generate with `cargo doc-all` (wrapper) or `cargo doc --workspace --no-deps --bins --all-features`
- **Examples**: Include code examples in doc comments where helpful
- **CLI**: Uses clap derive with rich doc comments for help text

### Safety and Unsafe Code

- **Unsafe**: Minimize usage; only when absolutely necessary (e.g., `fork`)
- **Pattern**: All `unsafe` blocks must have a SAFETY comment explaining why it's sound
- **Note**: The project includes `#![deny(unsafe_code)]` at workspace level; exceptions require explicit `#[expect(unsafe_code, reason = "...")]`

### Project Structure

- **Workspace**: Root Cargo.toml manages multiple crates in `crates/`
- **Binary**: `ocelot/` crate produces the main binary
- **Library crates**:
  - `crates/entry` - process supervisor
  - `crates/idle` - minimalist PID 1 for holding namespaces
  - `crates/supervise` - advanced supervisor with reaper/supervisor workers
  - `crates/zombie` - zombie process generator for testing
  - `crates/test-utils` - shared test utilities
- **Versioning**: Workspace-managed; uses `shadow-rs` for build-time version info

### Commit Messages

- Uses commitlint for validation
- Follows Conventional Commits format
- See `.github/workflows/lints.yaml` for configuration

### Additional Tools

- **Formatting**: Also uses `treefmt` to format all files: prettier (JSON/MD/JS/TS), taplo (TOML), nixfmt (Nix), shfmt + shellcheck (Shell), hclfmt (HCL)
- **Static analysis**: Codespell for typo detection
- **Shell completions**: Generate with `cargo run -- completions <shell>` (e.g., zsh, bash, fish)

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

# Build release binary
cargo build --release

# Generate shell completions
cargo run -- completions zsh > /etc/zsh/completions/_ocelot
```
