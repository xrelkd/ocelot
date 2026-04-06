## ADDED Requirements

### Requirement: execute_supervise SHALL follow phased execution order

The `execute_supervise` function SHALL execute phases in this order:

1. `mount::virtual_filesystems()` — proc/sys/dev/devpts/devshm/tmp
2. `clock::pre()` — RTC sync
3. `sysctl::pre()` — kernel parameters
4. `tmpfiles::pre()` — temporary directories
5. `symlinks::pre()` — boot tool symlinks
6. `environment::pre()` — environment variables
7. `modules::pre()` — kernel module loading
8. `network::pre()` — network setup (stub)
9. `mounts::pre()` — mount at /newroot + target, return target list
10. `hooks::pre()` — pre-mount hooks (decrypt, LVM, etc.)
11. `switch_root::only()` — pivot_root, no exec
12. `hooks::post()` — post-switch hooks
13. `symlinks::post()` — application symlinks
14. `environment::post()` — runtime environment
15. `tmpfiles::post()` — runtime temp directories
16. `sysctl::post()` — runtime kernel parameters
17. `mounts::post()` — mount at target directly
18. `network::post()` — runtime network (stub)
19. `modules::post()` — runtime modules (stub)
20. `security::post()` — security policies (stub)
21. `clock::post()` — NTP sync (stub)
22. `script::execute()` — boot script
23. `switch_root::exec_supervise()` — handoff to supervise (never returns)

#### Scenario: Full preSwitch execution

- **WHEN** `execute_supervise` is called with a config containing all preSwitch subsystems
- **THEN** all preSwitch phase functions execute before `switch_root::only()`

#### Scenario: Boot script executes before handoff

- **WHEN** a boot script is configured in `post_switch.handoff`
- **THEN** the script executes AFTER `switch_root::only()` but BEFORE `switch_root::exec_supervise()`

### Requirement: execute_shell SHALL follow same phased order with shell handoff

The `execute_shell` function SHALL follow the same phased execution order as `execute_supervise`, ending with `switch_root::only()` → boot_script → `switch_root::exec_shell()` instead of `switch_root::exec_supervise()`.

#### Scenario: Shell mode boot script timing

- **WHEN** `execute_shell` is called with a boot script configured
- **THEN** the boot script executes after switch_root but before the shell spawns

### Requirement: phase/ module SHALL organize subsystem pre/post functions

The `crates/bootstrap/src/phase/` directory SHALL contain: `mod.rs`, `clock.rs`, `sysctl.rs`, `tmpfiles.rs`, `symlinks.rs`, `environment.rs`, `modules.rs`, `network.rs`, `mounts.rs`, `hooks.rs`, `security.rs`, `handoff.rs`. Each file exports `pre()` and/or `post()` functions as appropriate.

#### Scenario: Phase module exports

- **WHEN** `use crate::phase` is imported in lib.rs
- **THEN** all subsystem pre/post functions are accessible

### Requirement: mounts::pre SHALL return mounted target paths

`mounts::pre()` SHALL return a `Vec<PathBuf>` of successfully mounted targets, to be passed to `mount_move_special()` as extra targets.

#### Scenario: Pre-mount returns target list

- **WHEN** `mounts::pre()` mounts two virtiofs shares at `/opt/data` and `/var/log`
- **THEN** it returns `[PathBuf::from("/opt/data"), PathBuf::from("/var/log")]`

### Requirement: mounts::post SHALL mount directly without /newroot prefix

`mounts::post()` SHALL mount filesystems at their target paths directly (no `/newroot` prefix), since switch_root has already occurred.

#### Scenario: Post-mount uses direct paths

- **WHEN** `mounts::post()` mounts a virtiofs share at `/var/log/app`
- **THEN** the mount target is `/var/log/app` (not `/newroot/var/log/app`)

### Requirement: Unused phase functions SHALL be suppressed

Phase functions for deferred subsystems (network, security, clock post, modules post) SHALL be implemented as stubs marked with `#[expect(dead_code, reason = "unsupported yet"]`.

#### Scenario: Network pre/post stubs do not trigger lint

- **WHEN** `cargo clippy` runs
- **THEN** no dead_code warnings for network phase stubs
