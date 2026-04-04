## MODIFIED Requirements

### Requirement: Bootstrap config structure

The system SHALL parse a YAML configuration file containing bootstrap-specific options including root filesystem, kernel modules, console, extra virtiofs mounts, symlinks, boot script, on-failure recovery, and an embedded supervise configuration section.

#### Scenario: Parse complete config

- **WHEN** a valid YAML config file is provided with all sections including `extra_virtiofs_mounts`, `symlinks`, and `boot_script`
- **THEN** all sections are parsed into the appropriate config structs

#### Scenario: Parse minimal config

- **WHEN** a YAML config file contains only the root section and supervise section
- **THEN** the config is parsed successfully with default values for omitted fields

#### Scenario: Parse config with extra virtiofs mounts

- **WHEN** the config specifies `extra_virtiofs_mounts` as a list of mount specifications
- **THEN** each mount is parsed with `tag`, `path`, and optional `with_overlay` fields

#### Scenario: Parse config with symlinks

- **WHEN** the config specifies `symlinks` as a list of symlink specifications
- **THEN** each symlink is parsed with `source` and `target` fields

#### Scenario: Parse config with boot script

- **WHEN** the config specifies `boot_script` with `command` and optional `args`, `on_failure`, and `working_directory`
- **THEN** the boot script config is parsed with all specified fields

#### Scenario: Parse config with module scan mode

- **WHEN** the config specifies `modules.mode: scan` with `modules.dir`
- **THEN** the module config is parsed in scan mode

#### Scenario: Parse config with module list mode

- **WHEN** the config specifies `modules.mode: list` with `modules.names` and optional `modules.dir`
- **THEN** the module config is parsed in list mode

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

### Requirement: Extra virtiofs mount config

The system SHALL support configuration of additional virtiofs mounts beyond the root filesystem, each with a tag, mount path, and optional overlay flag.

#### Scenario: Single extra mount config

- **WHEN** the config specifies one entry in `extra_virtiofs_mounts`
- **THEN** the mount is parsed with `tag`, `path`, and `with_overlay` (defaulting to false)

#### Scenario: Extra mount with overlay

- **WHEN** the config specifies `extra_virtiofs_mounts` with `with_overlay: true`
- **THEN** the mount is configured to use overlayfs with the virtiofs share as lower layer

#### Scenario: Extra mount with mount options

- **WHEN** the config specifies `extra_virtiofs_mounts` with `options`
- **THEN** the mount options are passed to the virtiofs mount syscall

### Requirement: Kernel module config

The system SHALL support kernel module configuration in two modes: explicit list mode and directory scan mode.

#### Scenario: Module list mode config

- **WHEN** the config specifies `modules.mode: list` with `modules.names`
- **THEN** each module name is parsed for loading via finit_module from the configured or default directory

#### Scenario: Module scan mode config

- **WHEN** the config specifies `modules.mode: scan` with `modules.dir`
- **THEN** the module config is parsed in scan mode with the specified directory

#### Scenario: Module list mode with custom directory

- **WHEN** the config specifies `modules.mode: list` with `modules.dir` and `modules.names`
- **THEN** modules are loaded from the specified directory

#### Scenario: Module list mode default directory

- **WHEN** the config specifies `modules.mode: list` without `modules.dir`
- **THEN** modules are loaded from `/lib/modules` by default

### Requirement: Symlink config

The system SHALL support symlink configuration with source and target paths.

#### Scenario: Single symlink config

- **WHEN** the config specifies one entry in `symlinks` with `source` and `target`
- **THEN** the symlink spec is parsed with both paths

#### Scenario: Multiple symlinks config

- **WHEN** the config specifies multiple entries in `symlinks`
- **THEN** each symlink spec is parsed in order

### Requirement: Boot script config

The system SHALL support boot script configuration with command, optional arguments, failure policy, and working directory.

#### Scenario: Boot script with command only

- **WHEN** the config specifies `boot_script.command`
- **THEN** the boot script is parsed with default `on_failure: warn` and no arguments

#### Scenario: Boot script with arguments

- **WHEN** the config specifies `boot_script.command` and `boot_script.args`
- **THEN** the boot script is parsed with the specified arguments

#### Scenario: Boot script with abort policy

- **WHEN** the config specifies `boot_script.on_failure: abort`
- **THEN** the boot script failure policy is set to abort

#### Scenario: Boot script with working directory

- **WHEN** the config specifies `boot_script.working_directory`
- **THEN** the boot script is configured to run in the specified directory

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
