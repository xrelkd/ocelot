## ADDED Requirements

### Requirement: BootstrapConfig deserialization works with serde_yaml::from_str

The `BootstrapConfig` struct SHALL be deserializable using `serde_yaml::from_str` directly (not just through `BootstrapConfig::load()`).

#### Scenario: Parse shell mode config directly

- **GIVEN** a YAML string with shell mode: `root:\n  type: virtiofs\n  tag: myfs\nconsole: ttyS0\nmode: shell\nprogram: /bin/sh`
- **WHEN** calling `serde_yaml::from_str::<BootstrapConfig>(&yaml)`
- **THEN** the result is `Ok(config)` with `config.mode` being `ExecutionMode::Shell`

#### Scenario: Parse supervise mode config directly

- **GIVEN** a YAML string with supervise mode: `root:\n  type: virtiofs\n  tag: myfs\nconsole: ttyS0\nmode: supervise\nprocesses: {}`
- **WHEN** calling `serde_yaml::from_str::<BootstrapConfig>(&yaml)`
- **THEN** the result is `Ok(config)` with `config.mode` being `ExecutionMode::Supervise`

#### Scenario: Parse config with environment variables directly

- **GIVEN** a YAML string containing `environmentVariables` field with key-value pairs
- **WHEN** calling `serde_yaml::from_str::<BootstrapConfig>(&yaml)`
- **THEN** the result includes the environment variables in `config.environment_variables`

#### Scenario: Parse config with working directory directly

- **GIVEN** a YAML string containing `workingDirectory` field
- **WHEN** calling `serde_yaml::from_str::<BootstrapConfig>(&yaml)`
- **THEN** the result includes the working directory in `config.working_directory`

#### Scenario: Backward compatibility with alternative format

- **GIVEN** a YAML string using `shell:` key directly (alternative format): `root:\n  type: virtiofs\n  tag: myfs\nshell:\n  program: /bin/sh`
- **WHEN** calling `serde_yaml::from_str::<BootstrapConfig>(&yaml)`
- **THEN** the result still works (backward compatible - either format is accepted)
