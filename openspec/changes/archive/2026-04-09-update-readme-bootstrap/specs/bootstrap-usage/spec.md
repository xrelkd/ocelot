## ADDED Requirements

### Requirement: Bootstrap command documented in Usage section

The README.md SHALL include a "The `bootstrap` Command" section in the Usage section, positioned after the supervise section and before the zombie section.

#### Scenario: Usage section includes bootstrap

- **WHEN** a user views the Usage section of README.md
- **THEN** they can see a description of the bootstrap command with its purpose and key features

### Requirement: Bootstrap workflow documented

The bootstrap section SHALL explain the three-tier phased architecture: pre-switch, switch-root, and post-switch phases.

#### Scenario: Bootstrap workflow explanation

- **WHEN** a user reads the bootstrap section
- **THEN** they understand what happens in each phase (pre-switch mounts virtual filesystems and configures system, switch-root performs pivot_root, post-switch continues configuration and hands off)

### Requirement: Bootstrap key features listed

The bootstrap section SHALL list key features including:

- Kernel modules loading
- Virtual filesystem mounting (procfs, sysfs, devpts, tmpfs, etc.)
- Root filesystem mounting (device, virtiofs, 9p, NFS, overlay)
- Boot script execution
- Handoff modes (supervise orchestrator, interactive shell, exec program)

> **Note**: The following features are NOT yet supported and SHALL NOT be mentioned in the README.md documentation:
>
> - Network configuration
> - Security module configuration (SELinux, AppArmor)
> - Clock configuration

#### Scenario: Bootstrap features listed

- **WHEN** a user reviews the bootstrap features
- **THEN** they can see a bulleted list of all supported bootstrap capabilities
