## Context

The bootstrap init system loads kernel modules via `finit_module` syscall. NixOS ships kernel modules as `.ko.xz` files (compressed with XZ/LZMA2). The kernel in our QEMU experiments has `CONFIG_MODULE_COMPRESS_XZ=y` but lacks `CONFIG_MODULE_DECOMPRESS`, meaning `finit_module` cannot accept compressed modules directly.

Current workaround: `pack-initrd.sh` decompresses `.ko.xz` → `.ko` at build time and uses `sed` to rewrite `modules.dep`. This creates a mismatch between config names (`.ko`) and actual NixOS disk layout (`.ko.xz`).

The `rotating_file` crate precedent shows that compression logic lives inline within its consumer crate (`compress.rs`, 21 lines) rather than as a separate crate. The same principle applies here — decompression is a preprocessing step for `finit_module`, not a standalone capability.

## Goals / Non-Goals

**Goals:**

- Bootstrap can load `.ko.xz` and `.ko.gz` modules at runtime without build-time decompression
- Pure Rust decompression — no C library dependencies
- Fallback-safe: uncompressed `.ko` modules continue to work unchanged
- Minimal binary size impact

**Non-Goals:**

- No support for `.ko.zst` (zstd) — not used by NixOS in our experiments
- No separate crate — decompression stays within `crates/bootstrap`
- No userspace `modprobe` replacement — dependency resolution already handled by `modules.dep` parsing
- No caching of decompressed modules — boot is a one-pass operation

## Decisions

### Decompression Library: `lzma-rs` over `xz2`

**Decision**: Use `lzma-rs 0.3.0` for XZ decompression, `flate2` (already in workspace) for GZ.

**Rationale**: `xz2` requires `liblzma` (C dependency via pkg-config), which breaks the pure-Rust goal and adds cross-compilation complexity. `lzma-rs` is pure Rust, well-maintained, and supports XZ/LZMA2 format used by NixOS. `flate2` with `rust_backend` feature is already a workspace dependency (used by `rotating_file`) and handles `.ko.gz` decompression.

**Alternatives considered**:

- `xz2` + `liblzma`: Smaller binary, faster decompression, but requires C toolchain and pkg-config.
- `xz4rust`: Smaller pure-Rust alternative, but less feature-complete and less maintained.

### Decompressed Data Storage: `memfd_create` over temp files

**Decision**: Use `nix::sys::memfd::memfd_create()` to create an anonymous in-memory file descriptor for decompressed module data.

**Rationale**: `memfd` creates a file descriptor backed by RAM, not disk. This avoids:

1. Temp file cleanup complexity on boot failure
2. Disk I/O overhead during boot
3. Security concerns with temp files containing kernel module code

The `nix 0.31` crate already supports `memfd_create` via the `"memfd"` feature, which just needs enabling in workspace `Cargo.toml`.

**Alternatives considered**:

- Temp files in `/tmp`: Simpler API but requires cleanup logic and adds disk I/O.
- `Vec<u8>` + `memfd_create` with `write`: Same approach, just confirming we write the decompressed bytes to the memfd before passing to `finit_module`.

### Module Format Detection: Extension-based

**Decision**: Detect module format by file extension (`.ko`, `.ko.xz`, `.ko.gz`) rather than magic bytes.

**Rationale**: Kernel module files have well-defined extensions. Extension-based detection is simpler, faster, and matches the existing code pattern in `modules.rs` (lines 17-24). Magic byte detection would add complexity for no practical benefit.

### Code Structure: Inline Module in `modules.rs`

**Decision**: Add decompression as a private module within `modules.rs` (or inline functions) rather than a separate `decompress.rs` file.

**Rationale**: The decompression code will be ~50-100 lines — smaller than most existing bootstrap modules. Following the `rotating_file` precedent (21-line `compress.rs` stays inline), there is no need for a separate file until the code grows beyond ~200 lines or has a second consumer.

**Proposed function structure**:

```rust
fn load_module_from_path(path: &str) -> Result<(), Error> {
    if path.ends_with(".ko.xz") {
        load_compressed_module(path, DecompressFormat::Xz)
    } else if path.ends_with(".ko.gz") {
        load_compressed_module(path, DecompressFormat::Gz)
    } else {
        load_uncompressed_module(path)
    }
}

fn load_compressed_module(path: &str, format: DecompressFormat) -> Result<(), Error> {
    let compressed = fs::read(path)?;
    let decompressed = decompress(&compressed, format)?;
    let fd = memfd_create("kmod")?;
    fd.write_all(&decompressed)?;
    fd.seek(SeekFrom::Start(0))?;
    finit_module(&fd, ...)?;
    Ok(())
}
```

## Risks / Trade-offs

| Risk                                              | Mitigation                                                                                            |
| ------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| `lzma-rs` decompression slower than `liblzma`     | Boot loads ~10-30 modules; decompression time is negligible vs. overall boot time                     |
| Large `.ko.xz` files consume RAM via memfd        | Kernel modules are typically <1MB each; total RAM usage during boot is minimal                        |
| `memfd_create` unavailable on old kernels         | Bootstrap runs on our controlled QEMU kernel (6.x+); not a concern. If needed, fallback to temp files |
| Binary size increase from `lzma-rs`               | ~50KB overhead; acceptable for a bootstrap binary                                                     |
| Decompression failure masks module loading errors | Error context chain preserves both decompression and `finit_module` error info                        |
