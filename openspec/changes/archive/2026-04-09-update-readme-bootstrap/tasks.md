## 1. Add bootstrap Usage section to README.md

- [x] 1.1 Add "The `bootstrap` Command" section after supervise section in Usage
- [x] 1.2 Document bootstrap workflow (three-tier phased architecture)
- [x] 1.3 List key features (modules, mounts, hooks, symlinks, sysctl, tmpfiles, handoff)
- [x] 1.4 Document configuration validation command
- [x] 1.5 Update Table of Contents to include bootstrap entry

> **Note**: Do NOT mention network, security, or clock features - not yet supported.

## 2. Add bootstrap CLI reference to README.md

- [x] 2.1 Add bootstrap main command help output
- [x] 2.2 Add bootstrap run subcommand help
- [x] 2.3 Add bootstrap config-template subcommand help (--mode: shell, supervise, exec)
- [x] 2.4 Add bootstrap validate subcommand help

## 3. Split Configuration Reference section

- [x] 3.1 Add brief introduction (2-3 sentences) explaining supervise vs bootstrap configs
- [x] 3.2 Add link to docs/supervise-config.md for full supervise config
- [x] 3.3 Add link to docs/bootstrap-config.md for bootstrap config
- [x] 3.4 Keep brief overview in README.md with links

## 4. Create docs/supervise-config.md

- [x] 4.1 Extract supervise config documentation from README.md
- [x] 4.2 Review and enhance if needed
- [x] 4.3 Add to docs/ directory

## 5. Create docs/bootstrap-config.md

- [x] 5.1 Document bootstrap configuration structure
- [x] 5.2 Document pre-switch phase config (modules, mounts, hooks, symlinks, sysctl, tmpfiles)
- [x] 5.3 Document switch-root phase config
- [x] 5.4 Document post-switch phase config
- [x] 5.5 Document handoff modes (supervise, shell, exec)
- [x] 5.6 Document config-template usage (--mode: shell, supervise, exec)

> **Note**: Do NOT document network, security, or clock configuration options - not yet supported.
