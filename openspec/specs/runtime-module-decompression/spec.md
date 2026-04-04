## ADDED Requirements

### Requirement: Runtime XZ decompression of kernel modules

The system SHALL decompress `.ko.xz` kernel module files at runtime using pure Rust XZ/LZMA2 decompression before passing the raw ELF data to `finit_module`.

#### Scenario: Load valid .ko.xz module

- **WHEN** a valid `.ko.xz` compressed kernel module file is requested for loading
- **THEN** the system decompresses the file in memory and passes the raw ELF to `finit_module` successfully

#### Scenario: Load corrupted .ko.xz module

- **WHEN** a `.ko.xz` file contains invalid or corrupted XZ data
- **THEN** the system returns a decompression error with context indicating the file path and decompression failure

#### Scenario: Load .ko.xz module with parameters

- **WHEN** a `.ko.xz` module is loaded with module parameters
- **THEN** the decompressed module is passed to `finit_module` with the specified parameters

### Requirement: Runtime GZIP decompression of kernel modules

The system SHALL decompress `.ko.gz` kernel module files at runtime using pure Rust GZIP decompression before passing the raw ELF data to `finit_module`.

#### Scenario: Load valid .ko.gz module

- **WHEN** a valid `.ko.gz` compressed kernel module file is requested for loading
- **THEN** the system decompresses the file in memory and passes the raw ELF to `finit_module` successfully

#### Scenario: Load corrupted .ko.gz module

- **WHEN** a `.ko.gz` file contains invalid or corrupted GZIP data
- **THEN** the system returns a decompression error with context indicating the file path and decompression failure

### Requirement: Memfd-backed module loading

The system SHALL use `memfd_create` to hold decompressed module data in memory, passing the resulting file descriptor to `finit_module` rather than creating temporary files on disk.

#### Scenario: Decompressed module loaded via memfd

- **WHEN** a compressed module is decompressed at runtime
- **THEN** the decompressed data is written to a memfd and the memfd is passed to `finit_module`

#### Scenario: Memfd creation failure

- **WHEN** `memfd_create` syscall fails (e.g., kernel does not support memfd)
- **THEN** the system returns an error with context indicating the memfd creation failure

### Requirement: Uncompressed module passthrough

The system SHALL load `.ko` (uncompressed) kernel module files directly via `finit_module` without any decompression or memfd overhead, preserving the existing behavior.

#### Scenario: Load uncompressed .ko module

- **WHEN** an uncompressed `.ko` kernel module file is requested for loading
- **THEN** the file is opened and passed directly to `finit_module` without decompression

#### Scenario: Mixed compressed and uncompressed modules

- **WHEN** a module list contains both `.ko.xz` and `.ko` files
- **THEN** each module is loaded using the appropriate path (decompressed or direct) based on its file extension

### Requirement: Pure Rust decompression dependencies

The system SHALL use pure Rust crates for decompression, avoiding C library dependencies such as `liblzma`.

#### Scenario: Build without C toolchain

- **WHEN** the project is built in an environment without `liblzma` or `pkg-config`
- **THEN** the build succeeds and decompression functionality is available
