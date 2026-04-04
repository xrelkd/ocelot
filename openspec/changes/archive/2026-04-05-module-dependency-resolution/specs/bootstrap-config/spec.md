## MODIFIED Requirements

### Requirement: Kernel module config

The system SHALL support kernel module configuration with an optional `modules.dep` file path for dependency resolution during config validation.

#### Scenario: Module list config with dependency file

- **WHEN** the config specifies `modules` as a list with `dep_file_path`
- **THEN** config validation parses the depfile, resolves dependency order via topological sort, and passes sorted names to bootstrap

#### Scenario: Module list config without dependency file

- **WHEN** the config specifies `modules` as a list without `dep_file_path`
- **THEN** modules are passed to bootstrap in user-specified order (existing behavior)

#### Scenario: Scan mode with dependency file

- **WHEN** the config specifies `modules` as a scan with `dep_file_path`
- **THEN** config validation resolves dependency order and passes sorted names to bootstrap

#### Scenario: Scan mode with dependency file and names filter

- **WHEN** the config specifies `modules` as a scan with `dep_file_path` and `names`
- **THEN** only the specified modules and their transitive dependencies are resolved and passed to bootstrap in sorted order

#### Scenario: Cyclic dependency in depfile

- **WHEN** the depfile contains cyclic dependencies among the modules to be loaded
- **THEN** config validation fails with a `CyclicDependency` error listing all modules in the cycle
