## ADDED Requirements

### Requirement: Module scan mode

The system SHALL support loading all kernel module files from a directory by scanning for `.ko`, `.ko.xz`, and `.ko.gz` files, in addition to the existing explicit module list mode.

#### Scenario: Scan mode loads all .ko files

- **WHEN** the config specifies `modules.mode: scan` with `modules.dir`
- **THEN** all `.ko` files in the directory are loaded via `finit_module()`

#### Scenario: Scan mode loads compressed modules

- **WHEN** the config specifies `modules.mode: scan` with `modules.dir`
- **THEN** `.ko.xz` and `.ko.gz` files are also loaded alongside `.ko` files

#### Scenario: Scan mode skips non-module files

- **WHEN** the directory contains files that are not kernel modules
- **THEN** only files matching `.ko`, `.ko.xz`, or `.ko.gz` extensions are processed

#### Scenario: Scan mode handles missing directory

- **WHEN** the specified `modules.dir` does not exist
- **THEN** module loading is skipped with an informational log message

#### Scenario: Scan mode individual module failure

- **WHEN** a module file fails to load during scan
- **THEN** a warning is logged and scanning continues with the next file

#### Scenario: Scan mode reports loading summary

- **WHEN** scan mode completes
- **THEN** a summary is logged showing loaded count, failed count, and total count

### Requirement: Module list mode (existing)

The system SHALL continue to support loading specific kernel modules by name from a configured directory.

#### Scenario: List mode loads named modules

- **WHEN** the config specifies `modules.mode: list` with `modules.names`
- **THEN** each named module is loaded in configuration order from `modules.dir`

#### Scenario: List mode default directory

- **WHEN** `modules.mode: list` is specified without `modules.dir`
- **THEN** modules are loaded from `/lib/modules` by default
