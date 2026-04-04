## Why

`finit_module` cannot load `.ko.xz` compressed kernel modules when the kernel lacks `CONFIG_MODULE_DECOMPRESS`. The current workaround decompresses modules at build time in `pack-initrd.sh`, creating a mismatch between config file names and the actual NixOS disk layout. Runtime decompression eliminates this gap and lets configs reference modules by their on-disk `.ko.xz` names.

## What Changes

- Add runtime `.ko.xz` and `.ko.gz` decompression in `crates/bootstrap/src/modules.rs` using pure Rust decompression crates
- Use `memfd_create` (via `nix` crate) to hold decompressed module data in memory, avoiding temp files
- Update `load_module_from_path` to detect compressed formats, decompress in-memory, and pass the memfd to `finit_module`
- Add `lzma-rs` dependency to `crates/bootstrap` and enable `"memfd"` feature for `nix` in workspace `Cargo.toml`

## Capabilities

### New Capabilities

- `runtime-module-decompression`: Bootstrap can decompress `.ko.xz` and `.ko.gz` kernel modules at runtime using pure Rust decompression and `memfd_create`, loading them via `finit_module` without build-time transformation.

### Modified Capabilities

<!-- None -->

## Impact

- `crates/bootstrap/src/modules.rs`: Add decompression logic to `load_module_from_path`
- `crates/bootstrap/Cargo.toml`: Add `lzma-rs` dependency
- `crates/bootstrap/src/error.rs`: Add `DecompressModule` error variant
- `Cargo.toml` (workspace): Enable `"memfd"` feature for `nix` crate
