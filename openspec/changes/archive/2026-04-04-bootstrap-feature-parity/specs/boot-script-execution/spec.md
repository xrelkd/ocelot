## ADDED Requirements

### Requirement: Boot script execution

The system SHALL optionally execute a boot script after switching root and before handing off to the supervise orchestrator or spawning a shell.

#### Scenario: Boot script executes successfully

- **WHEN** the config specifies `boot_script.command` and the script exits with code 0
- **THEN** the boot flow continues to the next stage (supervise handoff or shell spawn)

#### Scenario: Boot script with arguments

- **WHEN** the config specifies `boot_script.command` and `boot_script.args`
- **THEN** the script is executed with the specified arguments

#### Scenario: Boot script with custom working directory

- **WHEN** the config specifies `boot_script.working_directory`
- **THEN** the script is executed with that directory as its current working directory

#### Scenario: Boot script inherits environment variables

- **WHEN** environment variables are set in the config
- **THEN** the boot script inherits all configured environment variables

### Requirement: Boot script failure handling

The system SHALL handle boot script failures according to the configured failure policy.

#### Scenario: Script failure with warn policy

- **WHEN** the boot script exits with non-zero and `boot_script.on_failure` is `warn` (default)
- **THEN** a warning is logged and the boot flow continues

#### Scenario: Script failure with abort policy

- **WHEN** the boot script exits with non-zero and `boot_script.on_failure` is `abort`
- **THEN** the boot flow aborts with an error

#### Scenario: No boot script configured

- **WHEN** `boot_script` is not specified in the config
- **THEN** the boot script stage is skipped without error
