# Contributing to Ocelot

> [Ocelot](https://github.com/xrelkd/ocelot) is a process supervisor and init system written in Rust.

## Nix Flake (Recommended)

This project uses [Nix Flakes](https://nixos.wiki/wiki/Flakes) for development environment. We **strongly recommend** using Nix to ensure consistent tooling across all contributors.

### Quick Start

```bash
# Enter the development environment
nix develop

# Build the project
cargo build

# Run tests
cargo nextest run
```

### Using direnv (Recommended for Faster Development)

[direnv](https://direnv.net/) automatically loads the Nix environment when you enter the project directory.

1. Install direnv: `nix profile install nixpkgs#direnv`
2. Add to your shell (bash/zsh/fish)
3. Allow direnv in the project directory: `direnv allow`

Then the development environment will load automatically every time you enter the project.

### Alternative: Without Nix

If you don't use Nix, you need to install the following tools manually:

**Required:**

- Rust (see `rust-toolchain.toml` for version)
- Cargo
- cargo-nextest
- pkg-config
- libgit2

**Recommended (for formatting/linting):**

- rustfmt
- treefmt
- nixfmt
- shfmt
- shellcheck
- taplo
- hclfmt
- prettier

```bash
# Clone the project
git clone https://github.com/xrelkd/ocelot.git
cd ocelot

# Build
cargo build
cargo build --release

# Run
cargo run --package ocelot
```

## Development Workflow

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run the main binary
cargo run --package ocelot
```

### Testing

```bash
# Run tests with cargo test
cargo test

# Or use cargo-nextest (faster, used in CI)
cargo nextest run
cargo nextest run --release

# Run all tests with retries (preferred wrapper command)
cargo nextest-all
```

### Code Quality

**Always run these before creating a commit:**

```bash
# Format code
cargo fmt --all --check
treefmt                     # Format all file types (Rust, JS, JSON, TOML, Nix, Shell, HCL)
nix fmt                     # If using Nix

# Run lints (fix all warnings/errors)
cargo clippy-all            # Wrapper: cargo clippy --workspace --all-targets
cargo clippy-all --fix      # Auto-fix where possible
```

**Keep commits clean:**

- Fix all lints and compilation errors in the same commit as your feature/fix
- Do NOT create separate commits for lint fixes
- Keep commit messages concise and meaningful

### Branch Naming

Create feature/fix branches from `develop`:

```bash
# Feature branch
git checkout -b feat/my-new-feature develop

# Fix branch
git checkout -b fix/bug-description develop
```

Supported prefixes: `feat/*`, `feature/*`, `fix/*`, `hotfix/*`, `release/*`, `ci/*`

### Creating a Pull Request

1. Run formatters (`cargo fmt --all --check`, `treefmt`, or `nix fmt`)
2. Run lints and fix all warnings/errors (`cargo clippy-all --fix`)
3. Ensure tests pass (`cargo nextest run` or `cargo nextest-all`)
4. Push your branch to your fork
5. Create a PR targeting the `develop` branch
6. Ensure all CI checks pass
7. Keep commits clean, concise, and follow the commit message convention

> [!TIP]
> See [AGENTS.md](AGENTS.md) for detailed coding assistant guidelines and additional commands.

---

## Commit Message Convention

This project follows [Conventional Commits](https://www.conventionalcommits.org/) specification.
Commit messages are validated by [commitlint](https://commitlint.js.org/) in CI.

### Format

```text
<type>(<scope>): <description> (#PR)
```

### Types

| Type       | Description                        |
| ---------- | ---------------------------------- |
| `feat`     | New feature                        |
| `fix`      | Bug fix                            |
| `refactor` | Code refactoring                   |
| `perf`     | Performance improvement            |
| `docs`     | Documentation updates              |
| `style`    | Code formatting (no logic changes) |
| `ci`       | CI/CD modifications                |
| `chore`    | Maintenance tasks                  |
| `build`    | Build system or dependencies       |
| `test`     | Test additions or improvements     |

### Scopes

The scope is optional but recommended when the change affects a specific component.

#### One Scope Per Commit

**Each commit should only affect one scope.** If your PR modifies multiple components, create separate commits for each one.

Good:

```bash
# Commit 1
fix(crates/entry): rename `splice` arguments

# Commit 2
fix(crates/supervise): fix PID 1 warning message
```

Bad:

```bash
# Don't do this
fix: fix lints and errors in crates/entry and crates/supervise
```

**Lint fixes should be part of the feature commit.** Do not create separate commits for fixing lints or compilation errors.

#### Common Scopes

| Scope               | Description                    |
| ------------------- | ------------------------------ |
| `ocelot`            | Main binary crate              |
| `crates/entry`      | Process supervisor crate       |
| `crates/supervise`  | Advanced supervisor crate      |
| `crates/idle`       | Minimalist PID 1 crate         |
| `crates/zombie`     | Zombie process generator crate |
| `crates/test-utils` | Shared test utilities crate    |
| `readme`            | README documentation           |
| `agents`            | AGENTS.md file                 |
| `gitignore`         | .gitignore updates             |
| `actions`           | GitHub Actions workflows       |
| `nix`               | Nix/NixOS modules              |

#### Scope-less Commits

Some commits don't require a scope:

- `style: apply formatter`
- `chore: update Rust formatter`
- `build(deps): update Cargo.lock`

### Examples

```bash
# Feature in multiple scopes - create separate commits
feat(crates/supervise): add LZ4 log compression
feat(ocelot): add log rotation compression

# Bug Fix
fix(crates/supervise): reset signal handlers before `exec`

# Refactor
refactor(ocelot): restructure configuration modules

# Performance
perf(crates/supervise): inline handle event method

# Documentation
docs(readme): update the `README.md` file

# CI/CD
ci(actions): update `prettier` installation path

# Chore
chore(gitignore): add ignore rules for AI tool configs

# Style (no scope needed)
style: apply formatter

# Dependency Update (usually automated)
build(deps): bump lz4_flex from 0.11.6 to 0.13.0 (#43)
```

### PR References

When submitting a PR that touches multiple scopes, use multiple commits:

```bash
# If your PR modifies both crates/entry and crates/supervise
fix(crates/entry): rename `splice` arguments
fix(crates/supervise): fix PID 1 warning message

# Then merge with "Merge pull request #123"
```

Always reference the PR number when applicable:

```text
feat(crates/supervise): add configurable termination grace period to reaper (#21)
```

---

## Additional Commands

### Cargo Wrapper Commands (Nix DevShell)

The Nix devshell provides wrapper commands that set appropriate defaults:

```bash
cargo clippy-all            # cargo clippy --workspace --all-targets
cargo nextest-all           # cargo nextest run with retries (NEXTEST_RETRIES=5)
cargo test-all              # Run all tests including ignored
cargo doc-all               # cargo doc --workspace --no-deps --bins --all-features
```

### Shell Completions

Generate shell completions for your preferred shell:

```bash
cargo run -- completions zsh   > /etc/zsh/completions/_ocelot
cargo run -- completions bash  > /etc/bash_completion.d/ocelot
cargo run -- completions fish  > ~/.config/fish/completions/ocelot.fish
```
