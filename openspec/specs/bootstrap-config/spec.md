## ADDED Requirements

### Requirement: Bootstrap config structure

The system SHALL parse a YAML configuration file containing bootstrap-specific options and an embedded supervise configuration section.

#### Scenario: Parse complete config

- **WHEN** a valid YAML config file is provided with root, modules, and supervise sections
- **THEN** all sections are parsed into the appropriate config structs

#### Scenario: Parse minimal config

- **WHEN** a YAML config file contains only the root section and supervise section
- **THEN** the config is parsed successfully with default values for omitted fields

### Requirement: Root filesystem config

The system SHALL support root filesystem configuration with type, device/tag, filesystem type, mount options, and overlay flag.

#### Scenario: Virtiofs root config

- **WHEN** the config specifies `root.type: virtiofs` with `tag`
- **THEN** the root config is parsed with type=virtiofs, source=tag

#### Scenario: Block device root config

- **WHEN** the config specifies `root.type: block` with `device` and `fstype`
- **THEN** the root config is parsed with type=block, source=device, fstype

#### Scenario: 9p root config

- **WHEN** the config specifies `root.type: 9p` with `tag`
- **THEN** the root config is parsed with type=9p, source=tag

#### Scenario: Invalid root type

- **WHEN** the config specifies an unsupported root type
- **THEN** config validation fails with a descriptive error

### Requirement: Kernel module config

The system SHALL support kernel module configuration with an optional module directory and a list of module names.

#### Scenario: Module list config

- **WHEN** the config specifies `modules` as a list of module names
- **THEN** each module name is parsed for loading via finit_module

#### Scenario: Module directory config

- **WHEN** the config specifies `modules.dir`
- **THEN** modules are loaded from the specified directory path

### Requirement: Console config

The system SHALL support console device configuration.

#### Scenario: Console device specified

- **WHEN** the config specifies `console: ttyS0`
- **THEN** the console device is set to `/dev/ttyS0`

#### Scenario: Default console

- **WHEN** no console is specified in the config
- **THEN** the default console `/dev/console` is used

### Requirement: On-failure config

The system SHALL support error recovery configuration with optional debug shell path.

#### Scenario: Debug shell configured

- **WHEN** the config specifies `on_failure.shell: /bin/sh`
- **THEN** a debug shell is spawned on fatal errors

#### Scenario: No failure recovery

- **WHEN** no `on_failure` section is present
- **THEN** the system loops infinitely on fatal errors

### Requirement: Supervise config embedding

The system SHALL embed the full supervise configuration under a `supervise` key, reusing the existing `SupervisorConfig` schema.

#### Scenario: Embedded supervise config

- **WHEN** the YAML file contains a `supervise` section with process definitions
- **THEN** the section is parsed using the existing `SupervisorConfig` deserializer

#### Scenario: Supervise config validation

- **WHEN** the embedded supervise config is invalid
- **THEN** validation fails before boot proceeds, with error details reported

### Requirement: Config file loading

The system SHALL load configuration from a file path specified via CLI argument, or from a default path.

#### Scenario: Config file via CLI

- **WHEN** `ocelot bootstrap --file /etc/ocelot/bootstrap.yaml` is invoked
- **THEN** the configuration is loaded from the specified path

#### Scenario: Default config path

- **WHEN** `ocelot bootstrap` is invoked without `--file`
- **THEN** the system attempts to load from `/etc/ocelot/bootstrap.yaml`

#### Scenario: Config file not found

- **WHEN** the specified config file does not exist
- **THEN** an error is returned and boot aborts

#### Scenario: Invalid YAML syntax

- **WHEN** the config file contains malformed YAML
- **THEN** a parse error is returned with line/column information
