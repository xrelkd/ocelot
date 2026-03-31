## ADDED Requirements

### Requirement: Bootstrap boot flow

The system SHALL provide a complete bootstrap boot flow that initializes the environment, mounts the root filesystem, switches root, and hands off to the supervise orchestrator.

#### Scenario: Successful boot sequence

- **WHEN** `ocelot bootstrap` (or `ocelot boot`) is executed as PID 1 in a QEMU VM with valid configuration
- **THEN** the system mounts virtual filesystems, loads kernel modules, mounts rootfs, switches root, and executes the supervise orchestrator

#### Scenario: PID 1 verification

- **WHEN** `ocelot bootstrap` is executed with a PID other than 1
- **THEN** a warning is logged but execution continues

### Requirement: Virtual filesystem mounting

The system SHALL mount the following virtual filesystems before any other operations: `/proc` (proc), `/sys` (sysfs), `/dev` (devtmpfs), and `/run` (tmpfs with mode 0755).

#### Scenario: Mount all virtual filesystems

- **WHEN** the boot flow starts
- **THEN** proc, sysfs, devtmpfs, and tmpfs are mounted at their standard paths

#### Scenario: Mount point creation

- **WHEN** a mount point directory does not exist
- **THEN** the system creates the directory with mode 0755 before mounting

### Requirement: Kernel module loading

The system SHALL load specified kernel modules using `finit_module()` syscall from the configured module directory.

#### Scenario: Load modules from list

- **WHEN** the config specifies a list of kernel modules
- **THEN** each module is loaded via `finit_module()` in order

#### Scenario: Module load failure

- **WHEN** a kernel module fails to load
- **THEN** a warning is logged and loading continues with the next module

#### Scenario: No modules configured

- **WHEN** no modules are specified in the config
- **THEN** module loading is skipped without error

### Requirement: Root filesystem mounting

The system SHALL mount the root filesystem to `/newroot` based on the configured storage backend type.

#### Scenario: Mount virtiofs root

- **WHEN** the config specifies `root.type: virtiofs` with a tag
- **THEN** the virtiofs share is mounted at `/newroot` with the given tag as source

#### Scenario: Mount virtio-blk root

- **WHEN** the config specifies `root.type: block` with a device path
- **THEN** the block device is mounted at `/newroot` with the specified filesystem type

#### Scenario: Mount 9p root

- **WHEN** the config specifies `root.type: 9p` with a tag
- **THEN** the 9p share is mounted at `/newroot` with the given tag as source

#### Scenario: Device not immediately available

- **WHEN** a block device is not yet available at mount time
- **THEN** the system retries for up to 30 seconds before failing

### Requirement: Overlay filesystem support

The system SHALL support overlayfs on top of read-only root filesystems, providing a writable upper layer.

#### Scenario: Enable overlay on virtiofs

- **WHEN** the config specifies `root.overlay: true` with a read-only backend
- **THEN** an overlayfs is mounted with the backend as lowerdir, a tmpfs upperdir, and workdir

#### Scenario: Overlay directory structure

- **WHEN** overlayfs is enabled
- **THEN** upper and work directories are created under `/run/overlay/`

### Requirement: Switch root

The system SHALL perform a switch_root operation by moving virtual filesystems to the new root, chrooting, and executing the real init process.

#### Scenario: Move virtual filesystems

- **WHEN** switch_root begins
- **THEN** /proc, /sys, /dev, and /run are moved from the old root to `/newroot` using `MS_MOVE`

#### Scenario: Chroot and exec

- **WHEN** virtual filesystems are moved
- **THEN** the process chdirs to `/newroot`, chroots to `.`, and execs the supervise entry point

### Requirement: Console setup

The system SHALL set up the console device specified in configuration for standard I/O.

#### Scenario: Console device configuration

- **WHEN** a console device is specified (e.g., `ttyS0`)
- **THEN** `/dev/<console>` is opened and dup2'd to stdin, stdout, and stderr

#### Scenario: Default console

- **WHEN** no console device is specified
- **THEN** `/dev/console` is used as the default

### Requirement: Error handling and recovery

The system SHALL handle fatal errors gracefully by logging to kmsg and optionally spawning a debug shell.

#### Scenario: Fatal error with debug shell

- **WHEN** a fatal error occurs and `on_failure.shell` is configured
- **THEN** a debug shell is spawned on the configured console device

#### Scenario: Fatal error without debug shell

- **WHEN** a fatal error occurs and no debug shell is configured
- **THEN** the system logs the error to kmsg and enters an infinite loop

### Requirement: Handoff to supervise

The system SHALL call `supervise::execute()` with the embedded supervise configuration after successfully switching root.

#### Scenario: Supervise handoff

- **WHEN** switch_root completes successfully
- **THEN** `supervise::execute()` is called with the supervise config from the YAML file

#### Scenario: Supervise config validation before handoff

- **WHEN** the supervise config is loaded
- **THEN** it is validated before switch_root, and boot aborts if invalid
