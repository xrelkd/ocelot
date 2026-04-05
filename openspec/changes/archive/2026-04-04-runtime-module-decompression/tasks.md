## 1. Dependencies

- [x] 1.1 Add `lzma-rs = "0.3"` to `crates/bootstrap/Cargo.toml`
- [x] 1.2 Enable `"memfd"` feature for `nix` in workspace `Cargo.toml` (not needed — `memfd` module is always available in nix 0.31)
- [x] 1.3 Ensure `flate2` with `rust_backend` feature is available in bootstrap (verify workspace dependency)

## 2. Core Decompression Implementation

- [x] 2.1 Add `DecompressFormat` enum (Xz, Gz) and `decompress_module(data: &[u8], format: DecompressFormat) -> Result<Vec<u8>, Error>` function in `modules.rs`
- [x] 2.2 Implement XZ decompression branch using `lzma_rs::xz_decompress`
- [x] 2.3 Implement GZ decompression branch using `flate2::read::GzDecoder`
- [x] 2.4 Add error variants to `error.rs` for `DecompressModule` (with path and format fields)

## 3. Memfd Integration

- [x] 3.1 Implement `write_module_to_memfd(data: &[u8]) -> Result<OwnedFd, Error>` using `nix::sys::memfd::memfd_create`
- [x] 3.2 Refactor `load_module_from_path` to dispatch: `.ko.xz` → decompress + memfd → finit_module, `.ko.gz` → decompress + memfd → finit_module, `.ko` → direct finit_module
- [x] 3.3 Ensure memfd is sought to position 0 before passing to `finit_module`

## 4. Testing & Verification

- [x] 4.1 Run `cargo fmt --all --check` and `cargo clippy-all`
- [x] 4.2 Run `cargo nextest-all` to verify unit tests pass
