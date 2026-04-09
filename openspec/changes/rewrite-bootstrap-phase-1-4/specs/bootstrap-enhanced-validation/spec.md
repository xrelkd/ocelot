## ADDED Requirements

### Requirement: Module dependency detection SHALL be applied during validation

The `BootstrapConfig::validate()` method SHALL call the existing module dependency resolution functions to validate and sort module lists when a `modules.dep` file is provided.

#### Scenario: Validate modules with dependency file

- **WHEN** `BootstrapConfig` contains `modules` with `dep_file_path` pointing to a valid modules.dep file
- **THEN** validation calls the module dependency resolution functions to sort the module list

#### Scenario: Validation fails on circular dependency

- **WHEN** modules.dep file contains circular dependencies
- **THEN** validation returns an error indicating invalid module dependencies

#### Scenario: Validation succeeds with valid dependencies

- **WHEN** modules.dep file contains valid dependency ordering
- **THEN** validation succeeds and module list is sorted accordingly

### Requirement: Process dependency detection SHALL be applied during validation

The `BootstrapConfig::validate()` method SHALL validate that all processes referenced in handoff configuration exist and are properly defined.

#### Scenario: Validate handoff processes exist

- **WHEN** `BootstrapConfig` contains handoff configuration with process definitions
- **THEN** validation ensures all referenced processes are defined in the supervise configuration

#### Scenario: Validation fails on undefined process

- **WHEN** handoff configuration references a process name not defined in supervise processes
- **THEN** validation returns an error indicating undefined process

#### Scenario: Validation succeeds with all processes defined

- **WHEN** all processes in handoff configuration are defined in supervise processes
- **THEN** validation succeeds

### Requirement: Validation SHALL maintain existing checks

All existing validation checks (environment variable duplicates, mode exclusivity) SHALL continue to function alongside the new dependency validations.

#### Scenario: Existing validations still work

- **WHEN** BootstrapConfig has duplicate environment variables
- **THEN** validation fails with duplicate environment variables error

#### Scenario: Mode exclusivity validation still works

- **WHEN** BootstrapConfig has both shell and supervise modes defined
- **THEN** validation fails with mutual exclusivity error

### Requirement: Validation error types SHALL be extended

New error variants SHALL be added to handle module dependency and process dependency validation failures.

#### Scenario: Module dependency error variant exists

- **WHEN** examining the Error enum in bootstrap configuration
- **THEN** it contains a variant for module dependency validation failures

#### Scenario: Process dependency error variant exists

- **WHEN** examining the Error enum in bootstrap configuration
- **THEN** it contains a variant for process dependency validation failures

### Requirement: Validation SHALL return early on first failure

Validation SHALL stop at the first validation failure encountered and return that error, rather than collecting multiple validation errors.

#### Scenario: First validation error is returned

- **WHEN** BootstrapConfig has multiple validation issues (e.g., duplicate env vars AND missing process)
- **THEN** validation returns the first error encountered in validation order
