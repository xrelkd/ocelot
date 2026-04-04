## ADDED Requirements

### Requirement: Module dependency file parsing

The system SHALL parse a Linux `modules.dep` text file and build a mapping of module basenames to their dependency basenames.

#### Scenario: Parse valid depfile with no dependencies

- **WHEN** the depfile contains entries with no dependencies (e.g., `kernel/foo.ko.xz:`)
- **THEN** those modules are mapped with empty dependency lists

#### Scenario: Parse valid depfile with dependencies

- **WHEN** the depfile contains entries with dependencies (e.g., `kernel/bar.ko.xz: kernel/foo.ko.xz`)
- **THEN** `bar.ko.xz` is mapped to depend on `foo.ko.xz`

#### Scenario: Parse depfile with mixed entries

- **WHEN** the depfile contains entries with and without dependencies
- **THEN** all entries are parsed correctly regardless of dependency count

#### Scenario: Parse empty depfile

- **WHEN** the depfile is empty
- **THEN** an empty dependency map is returned

#### Scenario: Parse non-existent depfile

- **WHEN** the specified depfile path does not exist
- **THEN** a `ParseModuleDependencyFile` error is returned during config validation

### Requirement: Topological sort of module loading order

The system SHALL compute a valid module loading order from the dependency graph using topological sort during config validation.

#### Scenario: Sort modules with linear dependencies

- **WHEN** modules A depends on B, and B depends on C
- **THEN** the resolved loading order is C, B, A

#### Scenario: Sort modules with shared dependencies

- **WHEN** modules A and B both depend on C
- **THEN** C appears before both A and B in the resolved order

#### Scenario: Sort modules with no dependencies

- **WHEN** all requested modules have no dependencies
- **THEN** any order is valid (stable order preferred)

#### Scenario: Sort with extra depfile entries

- **WHEN** the depfile contains modules not in the user's requested list
- **THEN** only the requested modules and their transitive dependencies are included in the resolved order

### Requirement: Cyclic dependency detection

The system SHALL detect cyclic dependencies in the module dependency graph during config validation and return an error listing all modules involved in the cycle.

#### Scenario: Detect two-module cycle

- **WHEN** module A depends on B and B depends on A
- **THEN** a `CyclicDependency` error is returned with cycle path `A → B → A`

#### Scenario: Detect three-module cycle

- **WHEN** module A depends on B, B depends on C, and C depends on A
- **THEN** a `CyclicDependency` error is returned with cycle path `A → B → C → A`

#### Scenario: Detect self-dependency

- **WHEN** a module lists itself as a dependency
- **THEN** a `CyclicDependency` error is returned with cycle path `A → A`

#### Scenario: No cycle in valid DAG

- **WHEN** the dependency graph is a valid directed acyclic graph
- **THEN** topological sort succeeds without error

### Requirement: Config-to-bootstrap conversion with sorted names

The system SHALL pass only the sorted `names` list to the bootstrap library after dependency resolution; the `dep_file_path` is not forwarded.

#### Scenario: List mode with depfile converts to sorted names

- **WHEN** `ModulesConfig::List` is configured with `names` and a valid `dep_file_path`
- **THEN** `From::from` produces `ocelot_bootstrap::ModulesConfig::List` with `names` in topologically sorted order and no `dep_file_path`

#### Scenario: Scan mode with depfile converts to sorted names

- **WHEN** `ModulesConfig::Scan` is configured with `dep_file_path` and optional `names`
- **THEN** `From::from` produces `ocelot_bootstrap::ModulesConfig::List` with all (or filtered) module names in topologically sorted order

#### Scenario: List mode without depfile passes through unchanged

- **WHEN** `ModulesConfig::List` is configured without `dep_file_path`
- **THEN** `From::from` produces `ocelot_bootstrap::ModulesConfig::List` with the original user-specified order
