## ADDED Requirements

### Requirement: switch_root::only SHALL use pivot_root by default

`switch_root::only(config)` SHALL perform:

1. `nix::mount::mount(None, "/", None, nix::mount::MsFlags::MS_REC | nix::mount::MsFlags::MS_PRIVATE, None)` — make root private
2. `mount_move_special(extra_targets)` — move proc/sys/dev/dev/pts/dev/shm/run
3. `nix::unistd::mkdir("/newroot/oldroot", nix::unistd::MODE_0o755)?` — create old root mount point
4. `nix::unistd::pivot_root("/newroot", "/newroot/oldroot")` — switch root
5. `nix::unistd::chdir("/")` — change to new root
6. `nix::mount::umount2("/oldroot", nix::mount::MntFlags::MNT_DETACH)` — lazy unmount old root
7. If `cleanupOldRoot` is true: `nix::unistd::rmdir("/oldroot")`

#### Scenario: Successful pivot_root

- **WHEN** `switch_root::only()` is called with `method: pivot_root`
- **THEN** the process root filesystem is switched via pivot_root and old root is cleaned up

#### Scenario: Chroot fallback

- **WHEN** `switch_root::only()` is called with `method: chroot`
- **THEN** the process uses nix::unistd::chdir + nix::unistd::chroot as fallback (existing behavior preserved)

### Requirement: switch_root SHALL set mount namespace isolation

Before any mount operations, `switch_root::only()` SHALL execute `nix::mount::mount(None, "/", None, nix::mount::MsFlags::MS_REC | nix::mount::MsFlags::MS_PRIVATE, None)` to prevent mount propagation to parent namespaces.

#### Scenario: Mount namespace isolation

- **WHEN** `switch_root::only()` begins execution
- **THEN** the root mount is set to private before any other operations

### Requirement: mount_move_special SHALL accept extra targets

`mount_move_special(extra_targets: &[PathBuf])` SHALL move the standard virtual filesystems (proc, sys, dev, dev/pts, dev/shm, run) and then iterate over `extra_targets` to move any additional mounts.

#### Scenario: Move standard and extra mounts

- **WHEN** `mount_move_special(&[PathBuf::from("/opt/data")])` is called
- **THEN** proc, sys, dev, dev/pts, dev/shm, run, and /opt/data are all moved to /newroot

### Requirement: switch_root error type SHALL include PivotRoot variant

The error enum SHALL have a `PivotRoot { source: nix::Error }` variant for pivot_root-specific failures.

#### Scenario: PivotRoot error on unsupported filesystem

- **WHEN** pivot_root fails because /newroot is not a mount point
- **THEN** the error is `Error::PivotRoot` with the underlying nix error

### Requirement: switch_root::exec_supervise SHALL hand off to orchestrator

`switch_root::exec_supervise(orchestrator_config)` SHALL call `ocelot_supervise::execute()` and never return on success.

#### Scenario: Supervise handoff

- **WHEN** `exec_supervise()` is called after switch_root
- **THEN** the supervise orchestrator takes over and the function does not return

### Requirement: switch_root::exec_shell SHALL hand off to interactive shell

`switch_root::exec_shell(console_device, shell_config)` SHALL spawn an interactive shell with the console as controlling terminal, then trigger shutdown on exit.

#### Scenario: Shell handoff and shutdown

- **WHEN** `exec_shell()` is called and the shell exits
- **THEN** `shutdown()` is called to power off/reboot

### Requirement: Legacy switch_root functions SHALL be removed entirely

The legacy `switch_root()` and `switch_root_shell()` functions SHALL be deleted from the codebase, not merely marked with `#[expect(dead_code)]`.

#### Scenario: Legacy functions are removed

- **WHEN** searching the codebase for `switch_root` and `switch_root_shell` function definitions
- **THEN** no instances are found in source files

### Requirement: All nix function calls SHALL use fully qualified names

Every function call from the nix crate SHALL use the fully qualified path (e.g., `nix::mount::mount`, `nix::unistd::chdir`) instead of `use` statements or partially qualified names.

#### Scenario: Mount function uses fully qualified name

- **WHEN** examining mount-related code in switch_root
- **THEN** all nix::mount::\* functions are called with full qualification

#### Scenario: Unistd function uses fully qualified name

- **WHEN** examining chdir/chroot-related code in switch_root
- **THEN** all nix::unistd::\* functions are called with full qualification

#### Scenario: No use statements for nix crate

- **WHEN** examining the imports in switch_root.rs
- **THEN** there are no `use nix::mount;` or `use nix::unistd;` statements
