## ADDED Requirements

### Requirement: Supervise configuration documentation extracted

A separate markdown file SHALL be created documenting the supervise configuration, extracted from README.md.

#### Scenario: Supervise config file created

- **WHEN** a user needs to configure supervise
- **THEN** they can reference docs/supervise-config.md for full configuration options

### Requirement: Supervise config sections documented

The supervise configuration documentation SHALL cover all existing sections from README.md:

- Configuration schema (version, processes)
- Process definition fields (program, arguments, environment, working directory, terminationGracePeriod)
- Readiness/liveness probes
- Restart policies
- Shutdown signals
- Log configuration

#### Scenario: Supervise config sections complete

- **WHEN** a user reads the supervise config documentation
- **THEN** they have the same information as currently in README.md plus additional detail if needed
