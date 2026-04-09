## ADDED Requirements

### Requirement: Virtiofs support pre-flight check

The system SHALL verify that the kernel supports the virtiofs filesystem before attempting any virtiofs mount operations.

#### Scenario: Virtiofs support detected

- **WHEN** `/proc/filesystems` contains `virtiofs`
- **THEN** the check passes and virtiofs mount operations proceed

#### Scenario: Virtiofs support not available

- **WHEN** `/proc/filesystems` does not contain `virtiofs`
- **THEN** the system returns a descriptive error indicating that `CONFIG_VIRTIO_FS` must be enabled

#### Scenario: Check runs before first virtiofs mount

- **WHEN** the boot flow includes any virtiofs mount (root or extra)
- **THEN** the support check is performed once before the first virtiofs mount attempt

#### Scenario: Check skipped for non-virtiofs roots

- **WHEN** the root filesystem type is `block` or `9p` and no extra virtiofs mounts are configured
- **THEN** the virtiofs support check is skipped
