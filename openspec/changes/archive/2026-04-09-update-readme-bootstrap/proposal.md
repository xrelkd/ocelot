## Why

The README.md documentation is missing documentation for the `ocelot bootstrap` subcommand, which is a key feature of Ocelot for serving as an initramfs init system for QEMU VMs. Users need to understand how to configure and use `ocelot bootstrap` to properly boot Linux systems in virtualized environments.

## What Changes

- Add new "The `bootstrap` Command" section to Usage, placed after supervise section
- Add CLI reference for bootstrap subcommand (similar to supervise)
- Split Configuration Reference section into supervise and bootstrap distinct sections
- Create separate configuration documentation files for supervise and bootstrap

## Capabilities

### New Capabilities

- `bootstrap-usage`: Document the bootstrap command workflow and usage in README.md
- `bootstrap-cli-ref`: Add CLI reference documentation for bootstrap subcommand
- `bootstrap-config-doc`: Create separate configuration guide for bootstrap command
- `supervise-config-doc`: Create separate configuration guide for supervise command (extracted from existing README)

### Modified Capabilities

- N/A (new documentation only)

## Impact

- README.md will be updated with new sections
- New markdown files will be created for configuration guides
- Table of Contents will be updated
