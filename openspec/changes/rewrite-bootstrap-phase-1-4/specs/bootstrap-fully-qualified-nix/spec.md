## ADDED Requirements

### Requirement: All nix function calls SHALL use fully qualified names

Every function call from the nix crate SHALL use the fully qualified path (e.g., `nix::mount::mount`, `nix::unistd::chdir`) instead of `use` statements or partially qualified names.

#### Scenario: Mount functions use fully qualified names

- **WHEN** examining any mount-related code in the bootstrap crate
- **THEN** all nix::mount::\* functions are called with full qualification (e.g., `nix::mount::mount`, `nix::mount::umount2`)

#### Scenario: Unistd functions use fully qualified names

- **WHEN** examining any unistd-related code in the bootstrap crate
- **THEN** all nix::unistd::\* functions are called with full qualification (e.g., `nix::unistd::chdir`, `nix::unistd::chroot`, `nix::unistd::pivot_root`)

#### Scenario: No use statements for nix crate modules

- **WHEN** examining the imports in any file in the bootstrap crate
- **THEN** there are no `use nix::mount;` or `use nix::unistd;` statements

#### Scenario: Other nix modules use fully qualified names

- **WHEN** examining code that uses other nix modules (e.g., nix::sys)
- **THEN** all functions are called with full qualification from their respective modules

### Requirement: Pivot_root function usage SHALL be correctly implemented

The pivot_root function SHALL be called as `nix::unistd::pivot_root(new_root, put_old)` with proper parameters.

#### Scenario: Correct pivot_root parameters

- **WHEN** calling pivot_root in switch_root functionality
- **THEN** the first parameter is the new root path and second is the put_old path

#### Scenario: Pivot_root error handling

- **WHEN** pivot_root fails
- **THEN** the error is properly caught and converted to the appropriate bootstrap error type

### Requirement: Mount function usage SHALL be correctly implemented

The mount function SHALL be called as `nix::mount::mount(source, target, fstype, flags, options)` with proper parameters.

#### Scenario: Correct mount parameters for virtual filesystems

- **WHEN** mounting proc/sys/dev/etc.
- **THEN** source is Some("proc"), target is "/proc", fstype is Some("proc"), flags are MsFlags::empty(), options is None

#### Scenario: Correct mount parameters with flags

- **WHEN** mounting with specific flags like MS_REC | MS_PRIVATE
- **THEN** the flags parameter contains the correct combination

#### Scenario: Mount error handling

- **WHEN** mount fails
- **THEN** the error is properly caught and converted to the appropriate bootstrap error type

### Requirement: Umount2 function usage SHALL be correctly implemented

The umount2 function SHALL be called as `nix::mount::umount2(target, flags)` with proper parameters.

#### Scenario: Correct umount2 parameters for lazy unmount

- **WHEN** performing lazy unmount of old root
- **THEN** target is "/oldroot" and flags is MntFlags::MNT_DETACH

#### Scenario: Umount2 error handling

- **WHEN** umount2 fails
- **THEN** the error is properly caught and converted to the appropriate bootstrap error type
