## ADDED Requirements

### Requirement: Parse config path from kernel command line

The system SHALL parse the `ocelot.config=<path>` parameter from the kernel command line at `/proc/cmdline`.

#### Scenario: Config path present in cmdline

- **WHEN** `/proc/cmdline` contains `ocelot.config=/path/to/config.yaml`
- **THEN** the config path `/path/to/config.yaml` is returned

#### Scenario: Config path not present in cmdline

- **WHEN** `/proc/cmdline` does not contain an `ocelot.config=` parameter
- **THEN** `None` is returned

#### Scenario: Empty cmdline

- **WHEN** `/proc/cmdline` is empty
- **THEN** `None` is returned

### Requirement: Public API for config path parsing

The system SHALL expose a public function to get the config file path from kernel command line.

#### Scenario: Call config path function

- **WHEN** the public function `get_config_path()` is called
- **THEN** it reads `/proc/cmdline` and returns the parsed config path or `None`
