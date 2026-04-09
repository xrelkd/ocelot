# QEMU Example

This directory contains an example demonstrating how to use Ocelot's `bootstrap` functionality to initialize a `QEMU` virtual machine.

## Overview

The example shows how Ocelot can act as an initramfs for QEMU VMs, handling:

- Kernel module loading
- Filesystem mounting (`procfs`, `sysfs`, `devpts`, `tmpfs`, `9p`)
- `switch_root` operation
- Handoff to supervisors, shells, or executables

## Directory Structure

- `config/` - Bootstrap configuration files for different modes (shell, supervise, exec)
- `rootfs/` - Minimal root filesystem that will be mounted in the VM
- `scripts/` - Helper scripts for building and packing the initramfs
- `shared/` - Directory shared between host and VM via 9p filesystem
- `Justfile` - Build and execution commands

## Usage

### Prerequisites

- Nix (for development environment)
- QEMU system emulator
- Docker (for Docker image rootfs option)

### Building and Running

See the [Justfile](Justfile) for available commands. Common workflows:

```bash
# Build ocelot with musl static linking
just build-ocelot

# Get dependencies (busybox, alpine rootfs, etc.)
just get-busybox
just get-alpine-rootfs

# Assemble rootfs with busybox
just setup-minimal supervise

# Pack initramfs
just pack-minimal-initrd supervise

# Boot QEMU with busybox rootfs in supervise mode
just boot-busybox supervise

# Or boot with alpine rootfs
just boot-alpine supervise

# Or boot with docker image rootfs (e.g., ubuntu:24.04)
just boot-container ubuntu:24.04 supervise
```

### Modes

The example supports three handoff modes after bootstrap completion:

- `shell`: Interactive shell
- `supervise`: Ocelot's process supervisor (default)
- `exec`: Execute a specific program

### Shared Directory

The `shared/` directory is mounted to `/mnt/shared` inside the VM via 9p filesystem, allowing easy file exchange between host and guest.

See the [shared directory README](shared/README.md) for usage examples.

## Cleanup

```bash
# Remove build artifacts (keeps rootfs/)
just clean

# Remove everything including rootfs/
just clean-all
```
