## ADDED Requirements

### Requirement: Bootstrap CLI reference documented

The README.md SHALL include detailed CLI reference for the bootstrap subcommand in the Command Line Interface section, after the supervise CLI reference.

#### Scenario: Bootstrap main command documented

- **WHEN** a user views the CLI reference for bootstrap
- **THEN** they can see the full help output including all subcommands (run, config-template, validate)

### Requirement: Bootstrap subcommands documented

The CLI reference SHALL document each bootstrap subcommand:

- `run`: Run bootstrap with configuration file
- `config-template`: Output configuration template (with --mode option for shell/supervise)
- `validate`: Validate configuration file (with --output option for human/json)

#### Scenario: Bootstrap subcommands shown

- **WHEN** a user runs `ocelot bootstrap --help` or subcommand help
- **THEN** they see documentation matching the actual CLI output
