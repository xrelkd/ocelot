## ADDED Requirements

### Requirement: Symlink creation during boot

The system SHALL create filesystem symlinks during the bootstrap boot flow based on configuration specifications.

#### Scenario: Create single symlink

- **WHEN** the config specifies one entry in `symlinks` with `source` and `target`
- **THEN** a symbolic link is created from `source` to `target` under the new root

#### Scenario: Create multiple symlinks

- **WHEN** the config specifies multiple entries in `symlinks`
- **THEN** each symlink is created in configuration order

#### Scenario: Symlink parent directory creation

- **WHEN** the parent directory of a symlink source does not exist
- **THEN** the system creates all parent directories before creating the symlink

#### Scenario: Symlink target does not exist

- **WHEN** a symlink's target path does not exist
- **THEN** the symlink is still created and a warning is logged

#### Scenario: Empty symlinks list

- **WHEN** `symlinks` is not specified or is an empty list
- **THEN** the symlink creation stage is skipped without error

### Requirement: Symlink error handling

The system SHALL handle symlink creation failures gracefully without aborting the boot flow.

#### Scenario: Individual symlink creation failure

- **WHEN** a symlink cannot be created (e.g., permission denied)
- **THEN** a warning is logged and symlink creation continues with the next entry
