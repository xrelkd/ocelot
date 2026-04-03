## Why

The current `BootstrapConfig` has two issues:

1. The serde deserialization doesn't work properly with `#[serde(tag = "mode")]` + `#[serde(flatten)]` combination when using `serde_yaml::from_str` directly - only works through `BootstrapConfig::load()`
2. Missing CLI subcommands for validating and generating config templates, unlike the supervise command
3. `ocelot bootstrap run` fails while `ocelot bootstrap` works - inconsistency

Additionally, the existing template file `templates/basic.yaml` is too generic and needs proper classification to separate from bootstrap templates. Also, it's too complex - we need simpler templates for learning purposes.

The supervise command already has `ocelot supervise validate` and `ocelot supervise config-template`, but bootstrap lacks equivalent functionality. Additionally, we should allow `ocelot bootstrap run` to work the same as `ocelot bootstrap`.

## What Changes

- **Restructure BootstrapConfig**: Drop the `ExecutionMode` enum, use explicit `shell` and `supervise` fields instead
- Create `BootstrapSuperviseConfig` for supervise mode (currently just uses `HashMap<String, ProcessConfig>`)
- Rename `ShellConfig` to `BootstrapShellConfig` for shell mode clarity
- Add mutual exclusivity validation: exactly one of `shell` or `supervise` must be set
- Fix serde deserialization to work with `serde_yaml::from_str` directly
- **Reorganize template files**: Move templates into command-specific subdirectories
- **Simplify supervise templates**: Create minimal, basic, and full templates with tiered complexity
- **Update supervise config-template**: Add `--template` flag to select minimal/basic/full
- **Add CLI subcommands** to bootstrap to match supervise pattern:
  - `ocelot bootstrap run` - Run bootstrap (default behavior, alias for just `bootstrap`)
  - `ocelot bootstrap validate --file <path>` - Validate bootstrap YAML configuration
  - `ocelot bootstrap config-template` - Output default configuration templates
- Support both shell mode and supervise mode templates

## Capabilities

### New Capabilities

- `bootstrap-config-cli`: Add CLI validation and template generation for bootstrap configuration
- `bootstrap-config-serde-fix`: Fix serde deserialization to work with direct `from_str` calls
- `bootstrap-run-alias`: Add `run` subcommand as explicit alias for default bootstrap behavior
- `template-reorganization`: Reorganize config templates into subdirectories by command
- `supervise-template-tiers`: Add minimal, basic, full template tiers for supervise with --template flag

### Modified Capabilities

- `supervise-config-template`: Updated to support `--template` flag for selecting template tier

## Impact

- **Files to modify**:
  - `ocelot/src/config/bootstrap.rs` (restructure, fix serde, add validation)
  - `ocelot/src/cli/bootstrap.rs` (add subcommands: run, validate, config-template)
  - `ocelot/src/cli/mod.rs` (update Bootstrap command to support subcommands)
  - `ocelot/src/config/shell.rs` (rename ShellConfig to BootstrapShellConfig)
  - `ocelot/src/cli/supervise.rs` (add --template flag to config-template)
  - `ocelot/src/config/supervise.rs` (update template paths, add template methods)
  - `ocelot/src/config/templates/` (reorganize template files)
- **New directories**:
  - `ocelot/src/config/templates/supervise/`
  - `ocelot/src/config/templates/bootstrap/`
- **Dependencies**: serde_yaml (already in use)
- **Tests**: New tests for validate and config-template commands
