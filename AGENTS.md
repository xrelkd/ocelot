# AGENTS.md

## Before Making Any Changes

Always read these files before starting work:

- **[conventions.md](conventions.md)** — Rust coding standards (imports, attributes, error handling, async patterns, testing)
- **[CONTRIBUTING.md](CONTRIBUTING.md)** — Development workflow, git conventions, commit message format

---

## Essential Commands

### Using Nix (strongly recommended)

```bash
# Enter dev shell
nix develop

# Build
cargo build              # Debug
cargo build --release    # Release
cargo build -p ocelot    # Main binary only

# Code quality
cargo fmt --all --check  # Check formatting
treefmt                  # Format all file types
cargo clippy-all         # Lint all targets (fix with --fix)
cargo doc-all            # Generate docs

# Testing
cargo nextest-all        # Preferred test runner (with retries)
cargo test <name>        # Run single test
cargo test --test <file> # Run single integration test
```

### Without Nix

Install Rust toolchain (from `rust-toolchain.toml`):

```bash
cargo build
cargo fmt --all --check
treefmt
cargo clippy --workspace --all-targets
cargo nextest run --workspace  # Requires cargo-nextest
cargo doc --workspace --no-deps --bins --all-features
```

---

## Critical Workflow

- **Branching**: Create feature/fix branches from `develop` using `feat/` or `fix/` prefixes
- **Commits**: Follow Conventional Commits with scope (e.g., `feat(crates/supervise): add feature`)
- **Lint fixes**: Must be included in feature commits, never separate commits
- **Pre-commit**: Format code, run lints, ensure tests pass

---

## Project Structure

- Workspace with root `Cargo.toml` managing crates in `crates/`
- Main binary: `ocelot/` crate
- Key library crates:
  - `crates/entry` - minimal init/process supervisor
  - `crates/idle` - minimalist PID 1 for holding namespaces
  - `crates/supervise` - advanced supervisor with reaper/supervisor workers
  - `crates/zombie` - zombie process generator for testing
  - `crates/test-utils` - shared test utilities
- Versioning: Workspace-managed via `shadow-rs` (build-time version info)

---

## Code Style Specifics

- **Edition**: 2024
- **Formatter**: rustfmt (100 columns, vertical trailing comma)
- **Imports**: Granularity "Crate", grouped StdExternalCrate, layout Mixed
- **Naming**: snake_case (modules/functions/vars), CamelCase (types), SCREAMING_SNAKE_CASE (constants)
- **Error handling**: Use `snafu` (v0.9); avoid `unwrap()`/`expect()` in library code
- **Unsafe code**: All `unsafe` blocks require `#[expect(unsafe_code, reason = "...")]` with SAFETY comment
- **Lint suppression**: Use `#[expect(lint_name, reason = "explanation")]` instead of `#[allow]`
- **Attributes order**: Gate → Transform → Derive → Config → Contract → Lint → Physical
- **Derive ordering**: Standard library → third-party, alphabetical within groups
- **Format strings**: Use implicit named arguments (Rust 1.58+) like `format!("{name}")`

---

## Async Patterns (Tokio)

- Task spawning: Clone only needed values into scoped block before `tokio::spawn`
- Cancellation: Use `tokio_util::sync::CancellationToken` exclusively
- RwLock: Minimize write guard lifetime with scoped blocks
- Select loops: Follow standard structure with cancellation check first

---

## Testing Specifics

- Unit tests: Inline `#[cfg(test)]` modules or separate `tests/` directory
- Integration tests: In `crates/*/tests/` as standalone executables
- Test fixtures: Located in `foo/fixtures/` alongside `foo/tests.rs`, loaded with `include_str!`
- Test utilities: Use `ocelot-test-utils` crate
- Preferred runner: `cargo nextest` (NEXTEST_RETRIES=5 configured in wrapper)

---

## Documentation

- Rustdoc: Use `///` for public items
- Generate: `cargo doc-all` or `cargo doc --workspace --no-deps --bins --all-features`
- Include code examples in doc comments where helpful
- CLI help: Uses clap derive with rich doc comments

---

## Shell Completions

Generate with:

```bash
cargo run -- completions zsh > /etc/zsh/completions/_ocelot
cargo run -- completions bash > /etc/bash_completion.d/ocelot
cargo run -- completions fish > ~/.config/fish/completions/ocelot.fish
```
