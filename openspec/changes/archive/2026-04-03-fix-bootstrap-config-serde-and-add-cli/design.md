## Context

The bootstrap configuration in ocelot has two issues that need addressing:

1. **Serde Deserialization Issue**: The current `BootstrapConfig` uses `#[serde(tag = "mode")]` combined with `#[serde(flatten)]` on the `ExecutionMode` enum. While this works when using `BootstrapConfig::load()` (which reads file then parses), it fails when using `serde_yaml::from_str` directly because serde_yaml doesn't properly handle the flatten + tag combination with the outer struct fields.

2. **Missing CLI Commands**: The supervise command has `ocelot supervise validate` and `ocelot supervise config-template`, but bootstrap lacks equivalent functionality. Users need a way to validate their bootstrap YAML files and generate templates.

3. **CLI Inconsistency**: `ocelot bootstrap run` fails with "unexpected argument 'run'" while `ocelot bootstrap` works. This is inconsistent with supervise which has `ocelot supervise` and `ocelot supervise run` working similarly.

4. **Template Organization**: The existing `templates/basic.yaml` is too generic and needs reorganization. It's also too complex (106 lines) for users just learning ocelot.

## Goals / Non-Goals

**Goals:**

- Restructure `BootstrapConfig` to use explicit `shell` and `supervise` fields instead of `ExecutionMode` enum
- Create `BootstrapSuperviseConfig` struct for supervise mode configuration
- Rename `ShellConfig` to `BootstrapShellConfig` for clarity
- Add mutual exclusivity validation (exactly one of `shell` or `supervise` must be set)
- Fix `BootstrapConfig` deserialization to work with direct `serde_yaml::from_str` calls
- Reorganize template files into command-specific subdirectories
- Create tiered supervise templates (minimal, basic, full)
- Update `ocelot supervise config-template` to support `--template` flag
- Add `ocelot bootstrap run` subcommand (explicit alias for default behavior)
- Add `ocelot bootstrap validate` command to validate bootstrap YAML files
- Add `ocelot bootstrap config-template` command to generate configuration templates
- Support both shell mode and supervise mode templates

**Non-Goals:**

- Not changing the bootstrap runtime behavior (that change is complete)
- Not adding new configuration fields (beyond what's needed for templates)
- Not modifying the existing bootstrap logic in `crates/bootstrap`

## Decisions

### 1. How to restructure BootstrapConfig?

**Decision**: Use explicit optional fields instead of enum with flatten.

**YAML format** (new):

```yaml
root:
  type: virtiofs
  tag: rootfs
shell:
  path: /bin/sh
  args: ["-l"]
supervise:
  processes:
    init:
      command: /sbin/init
```

**Rationale**: The `#[serde(tag = "mode")]` + `#[serde(flatten)]` combination creates ambiguity for serde_yaml. Using explicit fields (`shell:` and `supervise:`) is straightforward, mutually exclusive, and works with direct `serde_yaml::from_str` calls.

### 2. New Structs

**Decision**: Create dedicated config structs for each mode:

- `BootstrapShellConfig`: wraps `ShellConfig` for shell mode
- `BootstrapSuperviseConfig`: wraps process definitions for supervise mode

**Rationale**: Provides clear separation and allows future expansion of mode-specific options.

### 3. Template File Reorganization

**Decision**: Organize templates into command-specific subdirectories with tiered complexity:

```
ocelot/src/config/templates/
├── supervise/
│   ├── minimal.yaml    # (new) - simplest HTTP server on port 55688
│   ├── basic.yaml      # (renamed from basic.yaml) - moderate complexity
│   └── full.yaml       # (renamed from basic.yaml) - production-ready
└── bootstrap/
    ├── shell.yaml      # (new) - shell mode template
    └── supervise.yaml  # (new) - supervise mode template
```

**Supervise Templates:**

1. **minimal.yaml** - Python HTTP server on port 55688:

   ```yaml
   version: "1.0"
   processes:
     http-server:
       program: python3
       arguments:
         - -m
         - http.server
         - "55688"
   ```

2. **basic.yaml** - Current basic.yaml (renamed, moderate complexity)
3. **full.yaml** - Full production-ready configuration (current basic.yaml renamed)

### 4. Supervise config-template --template flag

**Decision**: Add `--template` option to select which template to output:

```bash
ocelot supervise config-template                    # Output basic template (default)
ocelot supervise config-template --template minimal # Output minimal template
ocelot supervise config-template --template basic   # Output basic template
ocelot supervise config-template --template full    # Output full template
```

**Implementation**:

```rust
#[derive(Clone, ValueEnum)]
pub enum TemplateTier {
    Minimal,
    Basic,
    Full,
}

impl Commands {
    ConfigTemplate {
        #[clap(long, default_value = "basic")]
        template: TemplateTier,
    },
}
```

**Rationale**: Users can now choose the template that matches their needs - minimal for quick testing, basic for learning, full for production.

### 5. CLI Command Structure

**Decision**: Follow the same pattern as `ocelot supervise`, but add `run` as explicit subcommand:

```
ocelot bootstrap              # Run bootstrap (same as current behavior)
ocelot bootstrap run         # Run bootstrap (explicit, same as above)
ocelot bootstrap validate    # Validate config file
ocelot bootstrap config-template  # Output template (default: shell mode)
ocelot bootstrap config-template --mode supervise  # Supervise mode template
```

**Rationale**: This provides consistency with existing CLI patterns.

### 6. Template Content

**Decision**: Templates should include the new `environment_variables` and `working_directory` fields (from the completed add-env-vars-working-dir change) to show users the full configuration options.

**Rationale**: Since we've added these fields, they should be in the template for users to see.

## Risks / Trade-offs

- **[Risk] YAML format change**: Existing configs using `mode: shell` or `mode: supervise` will break. → **Mitigation**: This is a breaking change, but it's needed to fix the serde issue and the old format was never officially documented.
- **[Risk] Template explosion**: Multiple template variants could confuse users. → **Mitigation**: Provide clear help text and sensible defaults (basic for supervise, shell for bootstrap).

## Migration Plan

1. Restructure BootstrapConfig to use explicit fields (breaking change)
2. Reorganize template files into subdirectories
3. Create tiered supervise templates (minimal, basic, full)
4. Update supervise config-template with --template flag
5. Add CLI commands (run, validate, config-template)
6. Update templates to include new fields
7. Document the new YAML format

**Rollback**: If issues arise, the old format could be supported via a custom deserializer, but this is not planned.

## Open Questions

- Q: Should we support the old YAML format for backward compatibility?
  A: No, we don't care about backward compatibility of YAML format for bootstrap this time.
