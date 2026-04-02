## MODIFIED Requirements

### Requirement: Console setup

The system SHALL set up the console device specified in configuration for standard I/O, establishing it as the controlling terminal for the session.

#### Scenario: Console device configuration

- **WHEN** a console device is specified (e.g., `ttyS0`)
- **THEN** `/dev/<console>` is opened and dup2'd to stdin, stdout, and stderr

#### Scenario: Controlling terminal setup

- **WHEN** the console device is opened for I/O
- **THEN** the system calls TIOCSCTTY on the console file descriptor to establish it as the controlling terminal

#### Scenario: Default console

- **WHEN** no console device is specified
- **THEN** `/dev/console` is used as the default

### Requirement: Overlay filesystem support

The system SHALL support overlayfs on top of read-only root filesystems, providing a writable upper layer with isolated directories per mount source.

#### Scenario: Enable overlay on virtiofs

- **WHEN** the config specifies `root.overlay: true` with a read-only backend
- **THEN** an overlayfs is mounted with the backend as lowerdir, a tmpfs upperdir, and workdir

#### Scenario: Overlay directory structure

- **WHEN** overlayfs is enabled
- **THEN** upper and work directories are created under `/run/overlayfs/{source}/` where source is the mount identifier (tag for virtiofs/9p, sanitized device name for block)

#### Scenario: Multiple overlay mounts

- **WHEN** multiple mounts use overlay
- **THEN** each mount has isolated upper/work directories under `/run/overlayfs/{source}/`
