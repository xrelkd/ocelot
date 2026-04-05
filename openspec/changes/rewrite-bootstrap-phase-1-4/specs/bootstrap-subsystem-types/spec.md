## ADDED Requirements

### Requirement: ModulesConfig SHALL support list and scan modes

`ModulesConfig` SHALL have two variants: `List { dir, names, dep_file_path }` for explicit module loading and `Scan { dir, dep_file_path, names }` for directory scanning. Both SHALL support dependency file path for ordering.

#### Scenario: List mode with dependency resolution

- **WHEN** `ModulesConfig::List` specifies `depFilePath`
- **THEN** module names are sorted by dependency order from the modules.dep file

#### Scenario: Scan mode with filter

- **WHEN** `ModulesConfig::Scan` specifies optional `names` filter
- **THEN** only matching modules from the directory scan are loaded

### Requirement: NetworkConfig SHALL define network configuration structure

`NetworkConfig` SHALL support `mode: dhcp | static`, `timeout`, `interfaces` (per-interface config with address/gateway/dns), and `firewall` (rules file path). Values that are not yet implemented SHALL be marked with `#[expect(dead_code, reason = "unsupported yet"]`.

#### Scenario: DHCP network config (unsupported yet)

- **WHEN** `NetworkConfig` specifies `mode: dhcp` with `timeout: 10s`
- **THEN** the config deserializes but the runtime function is marked `#[expect(dead_code, reason = "unsupported yet"]`

#### Scenario: Static network config (unsupported yet)

- **WHEN** `NetworkConfig` specifies per-interface static addresses
- **THEN** the config deserializes but the runtime function is marked `#[expect(dead_code, reason = "unsupported yet"]`

### Requirement: HookSpecConfig SHALL define hook execution parameters

`HookSpecConfig` SHALL have fields: `name`, `command`, `arguments`, `timeout`, and `onFailure` (warn | abort). The runtime `HookSpec` SHALL convert to executable specifications.

#### Scenario: Hook with abort on failure

- **WHEN** a hook specifies `onFailure: abort`
- **THEN** hook failure causes the bootstrap process to return an error

#### Scenario: Hook with warn on failure

- **WHEN** a hook specifies `onFailure: warn`
- **THEN** hook failure logs a warning but bootstrap continues

### Requirement: SysctlConfig SHALL define kernel parameter settings

`SysctlConfig` SHALL be a `HashMap<String, serde_yaml::Value>` in the serialization layer and `HashMap<String, String>` in the runtime layer. Values SHALL be converted to strings at parse time.

#### Scenario: Sysctl parameter setting

- **WHEN** `sysctl` specifies `kernel.panic: 10`
- **THEN** the value is written to `/proc/sys/kernel/panic` during the appropriate phase

### Requirement: TmpfileConfig SHALL define temporary file/directory creation

`TmpfileConfig` SHALL have fields: `path`, `mode` (octal string), `type` (file | directory), `user`, `group`. User/group handling SHALL be marked with `#[expect(dead_code, reason = "unsupported yet"]`.

#### Scenario: Create temporary directory

- **WHEN** `tmpfiles` specifies `{ path: /run/lock, mode: "0755", type: directory }`
- **THEN** the directory is created with the specified permissions

### Requirement: SecurityConfig SHALL define security policy structure

`SecurityConfig` SHALL have optional `selinux` (enabled, policyPath, mode) and `apparmor` (enabled, profilesDir) fields. Implementation SHALL be marked with `#[expect(dead_code, reason = "unsupported yet"]`.

#### Scenario: SELinux config (unsupported yet)

- **WHEN** `security.selinux` specifies `enabled: true, mode: enforcing`
- **THEN** the config deserializes but the runtime function is marked `#[expect(dead_code, reason = "unsupported yet"]`

### Requirement: ClockConfig SHALL define clock synchronization settings

`ClockConfig` SHALL have `rtcSync: bool` and optional `ntp` (enabled, servers, timeout). RTC sync SHALL be implemented in pre-switch; NTP SHALL be marked with `#[expect(dead_code, reason = "unsupported yet"]`.

#### Scenario: RTC sync in pre-switch

- **WHEN** `clock.rtcSync: true` is set
- **THEN** the system clock is synchronized from the RTC hardware clock

#### Scenario: NTP sync (unsupported yet)

- **WHEN** `clock.ntp` specifies NTP servers
- **THEN** the config deserializes but the runtime function is marked `#[expect(dead_code, reason = "unsupported yet"]`

### Requirement: HandoffConfig SHALL define handoff parameters

`HandoffConfig` SHALL have `mode` (supervise | shell), optional `bootScript`, optional `supervise` (process definitions), and optional `shell` (program, arguments).

#### Scenario: Supervise handoff with boot script

- **WHEN** `handoff` specifies `mode: supervise` with `bootScript` and `supervise.processes`
- **THEN** the boot script executes first, then supervise orchestrator starts

#### Scenario: Shell handoff

- **WHEN** `handoff` specifies `mode: shell` with `shell.program: /bin/sh`
- **THEN** an interactive shell is spawned after all post-switch phases

### Requirement: ShutdownConfig SHALL define shutdown parameters

`ShutdownConfig` SHALL have fields: `timeout` (Duration), `sync: bool`, `umountAll: bool`. Implementation SHALL extend the existing `shutdown()` function.

#### Scenario: Shutdown with sync and unmount

- **WHEN** `shutdown` specifies `sync: true, umountAll: true`
- **THEN** `sync()` is called and all mounted filesystems are unmounted before reboot

### Requirement: OnFailurePolicy SHALL serialize as string

`OnFailurePolicy` SHALL serialize/deserialize as lowercase strings: `"warn"` and `"abort"`.

#### Scenario: Deserialize warn policy

- **WHEN** YAML specifies `onFailure: warn`
- **THEN** the policy deserializes to `OnFailurePolicy::Warn`

### Requirement: Unused subsystem fields SHALL be suppressed

All fields and functions for deferred implementations (network, security post, clock NTP, modules post, retry policy) SHALL be marked with `#[expect(dead_code, reason = "unsupported yet"]` (without phase references).

#### Scenario: No dead_code warnings for reserved subsystems

- **WHEN** `cargo clippy` runs on the bootstrap crate
- **THEN** no dead_code warnings for any reserved subsystem fields or functions
