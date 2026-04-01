# Rust Coding Conventions

Reusable conventions beyond the official Rust style guide for Rust projects.

## Edition

- Target Rust edition 2024.

## Project Structure

- Prefer `foo/mod.rs` + `foo/bar.rs` over `foo.rs` + `foo/bar.rs` for submodules.
- Use `include_str!`/`include_bytes!` for large embedded text, JSON, or binary data instead of inline literals.

### Source file ordering

Order elements within a source file as follows:

1. **Module declarations** (`mod foo;`, `#[cfg(test)] mod tests;`) — file pointers, no braces
2. **`use` statements** (5 groups, see below)
3. **Items** — types, traits, impls, functions, inline modules (`mod foo { ... }`)
4. **Inline test module** (`#[cfg(test)] mod tests { ... }`) — only for short tests (<100 lines)

The rule: **no braces = declaration = top. Braces = code = items section.**

```rust
// 1. Module declarations (file pointers, no blank lines between them)
#[cfg(test)]
mod tests;
mod submodule;

// 2. Imports
use std::collections::HashMap;

use serde::Serialize;

use self::Reaper;
use crate::config::Config;

// 3. Items (including inline module definitions)
mod sealed {
    pub trait Sealed {}
}

pub struct Foo;

impl Foo { /* ... */ }
```

When tests exceed ~100 lines and are split to `foo/tests.rs`, the `#[cfg(test)] mod tests;` declaration moves to the top with other declarations (it's now a file pointer, not a code block).

### No glob imports

Never use glob (wildcard) imports (`use foo::*`), including in test modules. Glob imports hide where names come from, make it unclear which items are actually used, and cause silent breakage when upstream adds a name that collides with a local one. Always import items explicitly:

```rust
// GOOD — explicit imports, clear provenance
use std::collections::HashMap;
use crate::config::Config;

// BAD — glob import
use std::collections::*;
use crate::config::*;
```

This applies everywhere — production code, test modules, benchmarks, examples.

### Import ordering

Five groups, separated by blank lines, alphabetical within each:

1. `std` crate
2. Third-party crates, and same-project sibling crates (e.g. `use my_common::...;` in a workspace)
3. `self::` paths
4. `crate::` paths

```rust
use std::collections::HashMap;
use std::sync::Arc;

use serde::Deserialize;
use tokio::sync::RwLock;
use my_common::telemetry::TracingConfig;

use self::{error, error::Error};

use crate::config::Config;
use crate::models::User;
```

### Module path conventions

- **`self::`** — prefer for child submodules. Shorter, refactoring-resilient, signals locality.
- **`crate::`** — use for siblings and other non-child modules.
- **`super::`** — restricted to `#[cfg(test)]` modules only, never in production code. See Testing section.

When importing from `self::`, use `use self::{module, module::Item}` — **not** `use self::module::{self, Item}`. These are not equivalent when `self::module` re-exports a macro:

```rust
// GOOD — self:: for child modules, works with macro re-exports
use self::{error, error::Error};

// BAD — breaks if `error` re-exports macros
use self::error::{self, Error};

// BAD — crate:: for your own child module is unnecessarily verbose
use crate::strategy::parameter::OrderPriceConfig; // when inside strategy/
```

## Dependencies

- Prefer the latest stable version of crates unless pinned for a specific reason.
- Never use crate versions with known CVEs or RUSTSEC advisories. Pin to a safe version if the latest is affected.
- Prefer well-maintained crates with recent activity. Never use abandoned or unmaintained crates.
- Run `cargo audit` before adding or upgrading dependencies.

## Style & Formatting

### Attribute ordering

One principle for all items — **Gate → Transform → Derive → Config → Contract → Lint → Physical**:

| #   | Category      | Applies to | Examples                                                                              |
| --- | ------------- | ---------- | ------------------------------------------------------------------------------------- |
| 1   | **Gate**      | All        | `#[cfg]`, `#[cfg_attr]`, `#[test]`, `#[tokio::test]`                                  |
| 2   | **Transform** | All        | `#[serde_as]`, `#[skip_serializing_none]`, `#[async_trait]`, `#[tracing::instrument]` |
| 3   | **Derive**    | Types      | `#[derive(...)]`                                                                      |
| 4   | **Config**    | Types      | Derive helpers: `#[serde(...)]`                                                       |
| 5   | **Contract**  | All        | `#[must_use]`, `#[deprecated]`, `#[non_exhaustive]`                                   |
| 6   | **Lint**      | All        | `#[expect(clippy::...)]`                                                              |
| 7   | **Physical**  | All        | Types: `#[repr]`. Functions: `#[inline]`, `#[cold]`, `#[no_mangle]`                   |

Within the same category, order alphabetically unless processing order requires otherwise.

**Key distinction:** `#[serde_as]` and `#[skip_serializing_none]` (from `serde_with`) are proc-macro **transforms** that rewrite the item before `#[derive]` processes it — they must precede `#[derive]`. They are not derive helpers like `#[serde(...)]`, which are consumed by derives and follow `#[derive]`.

```rust
// GOOD — types: gate → transform → derive → config → contract → physical
#[cfg(feature = "api")]
#[serde_as]
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
#[repr(u8)]
pub enum OrderStatus {

// GOOD — common case: no transforms, derive is first
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[repr(u8)]
pub enum PriceSource {

// GOOD — trait: gate → transform
#[cfg_attr(test, automock)]
#[async_trait]
pub trait OrderGateway {

// GOOD — functions: gate → transform → contract → lint → physical
#[cfg(feature = "metrics")]
#[tracing::instrument(skip_all, fields(symbol = field::display(&symbol)))]
#[must_use]
#[expect(clippy::too_many_arguments, reason = "matches exchange wire format")]
#[inline]
fn encode_order(/* ... */) -> Vec<u8> {

// GOOD — test: gate → transform
#[tokio::test]
#[tracing::instrument(skip_all)]
async fn test_order_submission() {

// BAD — serde_as is a transform, must precede derive
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde_as]
pub struct Response {

// BAD — repr between derive and config breaks grouping
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[repr(u8)]
#[serde(rename_all = "camelCase")]
pub enum PriceSource {

// BAD — physical before lint, transform after lint
#[inline]
#[expect(clippy::too_many_arguments, reason = "...")]
#[tracing::instrument(skip_all)]
fn process(/* ... */) {
```

### `derive` ordering

Order derive macros: **standard library → third-party**, alphabetical within each group:

```rust
// GOOD
#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, Deserialize, Serialize)]

// BAD — random order
#[derive(Serialize, Hash, Clone, Debug, Default)]
```

### Inline format syntax

Use modern inline format syntax — not positional placeholders:

```rust
// GOOD
format!("{e}")
format!("{name} has {count} items")

// BAD
format!("{}", e)
format!("{} has {} items", name, count)
```

### Linting

Run `cargo clippy` before committing. Treat all default warnings as errors in CI (`-D warnings`).

`#[allow(...)]` is **prohibited**. When a lint must be locally bypassed, use `#[expect(...)]` with a `reason` that explains why the suppression is correct for that specific site. Bare `#[allow(...)]` without a reason is a build-breaking violation:

```rust
// GOOD
#[expect(clippy::too_many_arguments, reason = "builder pattern not worth it for internal-only fn")]
fn create_session(/* ... */) {}

// BAD — #[allow] without reason
#[allow(clippy::too_many_arguments)]
fn create_session(/* ... */) {}
```

### Unsafe code policy

`unsafe` blocks must be gated behind `#[expect(unsafe_code, reason = "...")]` where the reason documents the invariant that makes the usage sound:

```rust
// GOOD
#[expect(unsafe_code, reason = "FFI call requires raw pointer; lifetime is bound to `self`")]
unsafe { ffi_init(ptr) }

// BAD — no expect gate or missing reason
unsafe { ffi_init(ptr) }
```

### Trait impl paths

When writing `impl` blocks for standard library traits, import and use the short trait name — not the fully qualified path:

```rust
use std::fmt;

// GOOD — import the module, use short path
impl fmt::Debug for MyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // ...
    }
}

impl fmt::Display for MyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // ...
    }
}

// BAD — fully qualified path is noisy
impl std::fmt::Debug for MyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // ...
    }
}
```

This applies to all standard library traits (`fmt::Debug`, `fmt::Display`, `ops::Deref`, `convert::From`, etc.). Import the parent module and use the short form.

## Error Handling

### Library

Use `snafu` (version 0.9) for all error handling. Add to `Cargo.toml`:

```toml
[dependencies]
snafu = { workspace = true }
```

### Error Enum Definition

Every crate defines its own `Error` enum in a dedicated `error.rs` module:

```rust
use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Failed to create epoll instance, error: {source}"))]
    CreateEpoll { source: nix::Error },

    #[snafu(display("Failed to parse signal number: {signal_num}"))]
    ParseSignal { signal_num: i32, source: nix::errno::Errno },

    #[snafu(display("Failed to execute child process"))]
    ExecuteChild,
}
```

**Key attributes:**

| Attribute                   | Purpose                                  |
| --------------------------- | ---------------------------------------- |
| `#[derive(Debug, Snafu)]`   | Derives Debug and Snafu macro            |
| `#[snafu(visibility(pub))]` | Makes generated context selectors public |
| `#[snafu(display("..."))]`  | Custom display messages                  |

### Error Variant Patterns

**Wrapping external errors with `source`:**

```rust
#[snafu(display("Failed to open config file: {}", filename.display()))]
OpenConfig { filename: std::path::PathBuf, source: std::io::Error },
```

**Contextual data without source:**

```rust
#[snafu(display("Cyclic dependency detected: {}", cycle.join(" -> ")))]
CyclicDependency { cycle: Vec<String> },
```

**Unit variants (no fields):**

```rust
#[snafu(display("Failed to execute child process"))]
ExecuteChild,
```

**Cross-crate error wrapping:**

```rust
#[snafu(display("{source}"))]
RunIdle { source: ocelot_idle::Error },
```

### Error Propagation

**Import `ResultExt` for `.context()`:**

```rust
use snafu::ResultExt;
```

**Pattern 1: Simple context selector:**

```rust
signal::sigprocmask(SigmaskHow::SIG_BLOCK, Some(&mask), None)
    .context(error::SetSignalMaskSnafu)?;
```

**Pattern 2: Context with extra fields:**

```rust
let sig_num_i32 = i32::try_from(sig_info.ssi_signo)
    .context(error::ConvertU32Snafu { value: sig_info.ssi_signo })?;
```

**Pattern 3: Lazy context with closure:**

```rust
let data = std::fs::read(&resolved_path)
    .with_context(|_| super::error::OpenConfigSnafu { filename: resolved_path.clone() })?;
```

**Pattern 4: Direct enum construction (for validation/business logic):**

```rust
return Err(Error::Validate {
    source: ValidationError::InvalidVersion { version: self.version.clone() },
});
```

**Pattern 5: Match-based error mapping:**

```rust
match wait::waitpid(pid, Some(WaitPidFlag::WNOHANG)) {
    Ok(WaitStatus::Exited(pid, exit_code)) => Ok(Some(ReapedProcess { pid, exit_code })),
    Ok(_) | Err(nix::Error::ECHILD) => Ok(None),
    Err(source) => Err(Error::WaitPid { source }),
}
```

### Cross-Crate Error Conversion

The top-level `ocelot` crate implements `From` for each library crate's error:

```rust
impl From<ocelot_idle::Error> for Error {
    fn from(source: ocelot_idle::Error) -> Self { Self::RunIdle { source } }
}
```

This enables transparent `?` operator usage:

```rust
ocelot_idle::execute()?;  // Automatically converts ocelot_idle::Error -> Error
```

### Prohibited Patterns

- **No custom error handling macros** — rely on snafu's derive macro

### `unwrap()` / `expect()` Exception

`unwrap()` and `expect()` are prohibited in production code **except** in the following scenario:

**Post-fork single-threaded context** — After `fork()` but before `execve()`, the process is single-threaded and the logic has already validated all inputs. At this point, operations like `CString::new()` on already-validated paths/strings are guaranteed to succeed. If they fail, it indicates a fundamental system invariant violation (e.g., null bytes in a path that was already checked), and panicking is the correct behavior because there is no sensible recovery path.

Example from `crates/supervise/src/command.rs`:

```rust
// GOOD — program path already validated by caller; null bytes here are impossible
let path_c = CString::new(self.program.as_os_str().as_bytes()).expect("Invalid program path");

// GOOD — environment variable strings from the host environment; null bytes are impossible
CString::new(pair).expect("Invalid environment variable")
```

The key criteria for this exception:

1. Code runs in a single-threaded post-fork context (or equivalent invariant-guaranteed context)
2. The `Result` is already logically guaranteed to be `Ok` by prior validation
3. If `Err` occurs, it indicates a system invariant violation with no recovery path
4. The `expect` message clearly documents what invariant was violated

Outside of this narrow exception, use proper error propagation with snafu `.context()`.

### Using `ensure!` and `fail!`

Snafu's `ensure!` and `fail!` macros are permitted for early returns and validation:

```rust
use snafu::{ensure, ResultExt};

// GOOD — ensure! for precondition checks
fn process(value: i32) -> Result<(), Error> {
    ensure!(value > 0, error::InvalidValueSnafu { value });
    // ...
}

// GOOD — fail! for explicit error returns
fn validate(config: &Config) -> Result<(), Error> {
    if config.entries.is_empty() {
        fail!(error::EmptyConfigSnafu);
    }
    // ...
}
```

Use `ensure!` for guard clauses and `fail!` when you need to construct and return an error explicitly. Prefer `.context()` when wrapping `Result` types from external calls.

### Test Utilities

The `ocelot-test-utils` crate uses `Box<dyn std::error::Error>` for flexibility:

```rust
pub type TestResult<T> = Result<T, Box<dyn std::error::Error>>;
```

## Generics & Trait Bounds

### `impl Trait` for simple cases

When a function has a single trait bound on a parameter and doesn't need the type name elsewhere, use `impl Trait` in argument position instead of a named generic:

```rust
// GOOD — simple, readable
fn process(reader: impl Read)
fn log(msg: impl Display)

// BAD — unnecessary generic when the type isn't referenced elsewhere
fn process<T>(reader: T) where T: Read
fn log<T: Display>(msg: T)
```

Use a named generic when you need to reference the type in multiple positions (return type, other parameters, struct fields) or when the bound is complex:

```rust
// Named generic needed — type appears in return position
fn parse<T: FromStr>(input: &str) -> Result<T, T::Err>

// Named generic needed — same type in multiple parameters
fn merge<T: Ord>(a: &[T], b: &[T]) -> Vec<T>
```

### Trait bound ordering

When a type has multiple bounds, order them as follows:

1. **Unsized relaxation** (`?Sized`)
2. **Traits** — alphabetical (`Clone`, `Display`, `Iterator`, `Serialize`)
3. **Auto traits** — alphabetical (`Send`, `Sync`, `Unpin`)
4. **Lifetimes** (`'a`, `'static`)

```rust
fn process<'a, D>(data: D)
where
    D: ?Sized + Clone + Display + Serialize + Send + Sync + Unpin + 'a,
```

For short bounds, inline syntax is fine. Use `where` clauses when bounds exceed one line:

```rust
// Inline — fits on one line
fn short<T: Clone + Send>(val: T)

// where clause — multiple bounds, stays readable
fn long<'a, T, U>(val: T, other: U) -> Result<()>
where
    T: ?Sized + Serialize + Send + Sync + 'a,
    U: Clone + Into<String>,
```

## Tracing & Logging

### Full-path macro calls

Always use the full path for tracing macros instead of importing them:

```rust
// GOOD
tracing::info!("message");
tracing::debug!(key = value, "message");
tracing::warn!(error = field::display(&e), "message");

// BAD — do not import tracing macros
use tracing::{info, debug, warn, error};
info!("message");
```

**Exception:** `tracing::instrument` may be imported directly because it's an attribute macro:

```rust
use tracing::{field, instrument};
```

### Field recording with `tracing::field`

Add `use tracing::field;` to files that use structured fields. Use `field::display()` for `Display` types and `field::debug()` for `Debug` types.

Never use `%` or `?` syntax sugar — it prevents `cargo fmt` from formatting tracing macro bodies.

Primitive fields (integers, booleans) are recorded directly without a wrapper, but must always use explicit `key = value` syntax — never shorthand:

```rust
use tracing::field;

// GOOD
tracing::info!(addr = field::display(&addr), "listening");
tracing::debug!(duration = field::debug(&duration), "elapsed");
tracing::info!(fetched = fetched, skipped = skipped, "done");

// BAD — implicit field shorthand, hard to read and fragile under renames
tracing::info!(fetched, skipped, "done");

// BAD — prevents cargo fmt from working
tracing::info!(%addr, "listening");

// BAD — unnecessarily verbose
tracing::info!(addr = tracing::field::display(&addr), "listening");
```

### Spans over repeated fields

Use `tracing::info_span!` or `#[tracing::instrument]` to carry context that applies to multiple log events, rather than repeating the same fields in every call:

```rust
// GOOD — span carries context for all events within it
#[tracing::instrument(skip_all, fields(user_id = id))]
async fn process_request(id: &str, ...) {
    tracing::info!("request started");      // inherits user_id
    tracing::info!("request_complete");     // inherits user_id
}

// GOOD — manual span
let span = tracing::info_span!("operation", key = field::display(&key));
let _enter = span.enter();
tracing::info!("success");                 // inherits key

// BAD — repeating fields in every event
tracing::info!(user_id = id, "request started");
tracing::info!(user_id = id, "request_complete");
```

## Async Patterns

### Task spawning

Clone only needed values into a scoped `{ }` block before `tokio::spawn`:

```rust
{
    let state = state.clone();
    let cancel = cancel.clone();
    handles.push(tokio::spawn(async move {
        // uses state, cancel
    }));
}
```

### Cancellation

Use `tokio_util::sync::CancellationToken` exclusively. No broadcast channels for shutdown.

All long-lived loops follow the same `tokio::select!` structure:

```rust
loop {
    tokio::select! {
        _ = cancel.cancelled() => {
            tracing::info!("shutting down");
            return Ok(());
        }
        _ = tokio::time::sleep(interval) => {
            // work
        }
    }
}
```

### RwLock write scope

Minimize write guard lifetime — use scoped blocks or explicit `drop()`:

```rust
{
    let mut guard = state.write().await;
    *guard = new_value;
}  // guard dropped here
```

## Naming

- Prefer full words over abbreviations: `quantity` not `qty`, `adjustment` not `adj`, `parameter` not `param`, `configuration` not `cfg`.
- Exceptions: widely understood abbreviations from the domain (`bbo`, `obi`, `cv`) and Rust conventions (`ctx`, `rx`/`tx`, `fn`, `impl`).

## Testing

### Test file organization

When tests exceed ~100 lines, split to `foo/tests.rs`. Keep short tests inline at the bottom of the file.

### Test fixtures

Shared test fixtures (TOML configs, JSON payloads, SQL schemas, etc.) live in `foo/fixtures/` alongside `foo/tests.rs` and are loaded with `include_str!`:

```rust
// src/config/tests.rs
const VALID_CONFIG: &str = include_str!("fixtures/valid_minimal.toml");
const FULL_CONFIG: &str = include_str!("fixtures/full.toml");
```

```text
src/config.rs
src/config/tests.rs
src/config/fixtures/
    valid_minimal.toml
    full.toml
```

Keep small, test-specific literals inline when the content's purpose is being wrong in a specific way — the defect should be visible next to the assertion:

```rust
// GOOD — inline: the point IS the missing field
let toml = r#"
[backend]
type = "openrouter"
base_url = "https://openrouter.ai/api/v1"
# api_key_env intentionally absent
"#;
assert!(matches!(result, Err(ConfigError::Validation(_))));

// GOOD — fixture file: valid baseline config reused across tests
const FULL_CONFIG: &str = include_str!("fixtures/full.toml");
```

Heuristic: if a fixture is shared across multiple tests or exceeds ~10 lines, extract it. If it's testing a specific defect, keep it inline.

### Test module imports

Test modules use `super::` to import from the parent module. Import items explicitly — no glob imports. Production code must use `crate::` or `self::` paths — never `super::`:

```rust
// GOOD — production code uses crate:: or self::
use crate::config::Config;
use self::{error, error::Error};

// GOOD — test module uses explicit super:: imports
#[cfg(test)]
mod tests {
    use super::{MyStruct, process, Config};
    use tokio::net::TcpStream;
}

// BAD — glob import in test module
#[cfg(test)]
mod tests {
    use super::*;
}

// BAD — super:: in production code
use super::utils::parse;
```
