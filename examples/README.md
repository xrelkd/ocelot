# Examples

This directory contains practical examples demonstrating how to use Ocelot in various scenarios.

## QEMU Example

The `qemu` subdirectory contains a complete example showing how to use Ocelot's bootstrap functionality to initialize a QEMU virtual machine. This demonstrates:

- Using `Ocelot` as an `initramfs` for `QEMU` VMs
- Loading kernel modules and mounting filesystems
- Performing `switch_root` and handing off to supervisors or shells
- Sharing directories between host and guest via `9p`

### Usage

See the [QEMU README](qemu/README.md) for detailed instructions on building and running the example.

### Modes

The example supports several bootstrap modes:

- `shell`: Hands off to an interactive shell after boot
- `supervise`: Hands off to Ocelot's supervise orchestrator
- `exec`: Hands off to a specified executable

## Purpose

Examples in this directory are meant to:

1. Showcase real-world usage patterns of Ocelot
2. Provide testbeds for new features
3. Demonstrate integration with other technologies (like QEMU)
4. Serve as starting points for users' own implementations
