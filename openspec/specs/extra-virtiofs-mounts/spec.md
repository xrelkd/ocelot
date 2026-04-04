## ADDED Requirements

### Requirement: Extra virtiofs mounts

The system SHALL support mounting multiple additional virtiofs shares beyond the root filesystem, each with an optional per-share overlayfs for writable access.

#### Scenario: Mount single extra virtiofs share

- **WHEN** the config specifies one entry in `extra_virtiofs_mounts` with `tag` and `path`
- **THEN** the virtiofs share is mounted at the specified path under the new root

#### Scenario: Mount multiple extra virtiofs shares

- **WHEN** the config specifies multiple entries in `extra_virtiofs_mounts`
- **THEN** each share is mounted at its respective path in configuration order

#### Scenario: Extra mount with overlay enabled

- **WHEN** an extra virtiofs mount has `with_overlay: true`
- **THEN** an overlayfs is mounted with the virtiofs share as lowerdir and a tmpfs-backed upperdir for writable access

#### Scenario: Overlay directory isolation per share

- **WHEN** multiple extra mounts have `with_overlay: true`
- **THEN** each share gets isolated upper/work directories under `/run/overlayfs/{tag}/`

#### Scenario: Extra mounts created after root filesystem

- **WHEN** the boot flow reaches the extra mounts stage
- **THEN** the root filesystem is already mounted at `/newroot` and extra mount paths are resolved relative to `/newroot`

#### Scenario: Mount point directory creation

- **WHEN** an extra mount's target path does not exist
- **THEN** the system creates all parent directories with mode 0755 before mounting

#### Scenario: Empty extra mounts list

- **WHEN** `extra_virtiofs_mounts` is not specified or is an empty list
- **THEN** the extra mounts stage is skipped without error

### Requirement: Extra virtiofs mount error handling

The system SHALL log warnings for individual extra mount failures without aborting the boot flow.

#### Scenario: Single extra mount failure

- **WHEN** one extra virtiofs mount fails
- **THEN** a warning is logged and the boot flow continues with remaining mounts

#### Scenario: All extra mounts fail

- **WHEN** all extra virtiofs mounts fail
- **THEN** warnings are logged for each and the boot flow continues to the next stage
