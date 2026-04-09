## ADDED Requirements

### Requirement: Bootstrap configuration documentation created

A separate markdown file SHALL be created documenting the bootstrap configuration file format.

#### Scenario: Bootstrap config file created

- **WHEN** a user needs to configure bootstrap
- **THEN** they can reference docs/bootstrap-config.md for full configuration options

### Requirement: Bootstrap config sections documented

The bootstrap configuration documentation SHALL cover:

- Pre-switch phase configuration (modules, mounts, hooks, symlinks, sysctl, tmpfiles)
- Switch-root phase (root filesystem mount spec)
- Post-switch phase (same as pre-switch plus handoff configuration)
- Handoff modes (supervise, shell, exec)

> **Note**: The following configuration options are NOT yet supported and SHALL NOT be mentioned in the documentation:
>
> - Network configuration (`network` field)
> - Security module configuration (`security` field)
> - Clock configuration (`clock` field)

#### Scenario: Bootstrap config sections complete

- **WHEN** a user reads the bootstrap config documentation
- **THEN** they understand all configuration options available for each phase

### Requirement: Bootstrap config template documented

The bootstrap configuration documentation SHALL show how to generate config templates with `ocelot bootstrap config-template --mode shell|supervise|exec`.

#### Scenario: Bootstrap config template usage shown

- **WHEN** a user wants to create a bootstrap configuration
- **THEN** they can generate a template using the config-template subcommand
