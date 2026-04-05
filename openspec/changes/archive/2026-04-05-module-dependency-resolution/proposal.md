## Why

Currently, `ocelot-bootstrap` loads kernel modules in user-specified order (List mode) or filesystem enumeration order (Scan mode), with no awareness of inter-module dependencies. If module A depends on module B but A is loaded first, `finit_module` fails and the module is silently skipped. Users must manually determine and specify the correct loading order, which is error-prone and requires kernel internals knowledge.

## What Changes

- Add `dep_file_path: Option<String>` to `ModulesConfig::List` in the ocelot binary's serde config
- Add `dep_file_path: String` (required) and `names: Option<Vec<String>>` to `ModulesConfig::Scan` in the ocelot binary's serde config
- In ocelot's config validation: parse `modules.dep`, build dependency graph via petgraph, topologically sort, detect cycles
- After validation, the `From::from` conversion passes only the sorted `names` list to bootstrap — `dep_file_path` is not forwarded
- Bootstrap's `ModulesConfig` remains unchanged in shape; doc comments added to clarify the assumption that `names` are provided in correct dependency order
- `load_modules` continues to return `()` (best-effort, warnings only) — no `Result` change
- Bootstrap receives zero new dependencies; ocelot reuses existing petgraph

## Capabilities

### New Capabilities

- `module-dependency-resolution`: Ocelot parses `modules.dep` files, validates module loading order via topological sort, detects cyclic dependencies, and passes pre-sorted module names to bootstrap

### Modified Capabilities

- `bootstrap-config`: The `modules` configuration schema in the ocelot binary changes — `ModulesConfig::List` gains an optional `dep_file_path` field; `ModulesConfig::Scan` gains a required `dep_file_path` field and an optional `names` field. Bootstrap library types unchanged except for documentation.

## Impact

- `ocelot/src/config/bootstrap.rs` — serde-enabled `ModulesConfig` gains new fields; validation adds depfile parsing + topological sort + cycle detection via petgraph; `From` impl updated
- `crates/bootstrap/src/config.rs` — no structural changes; doc comments added to `ModulesConfig` clarifying the pre-sorted order assumption
- `crates/bootstrap/src/modules.rs` — no structural changes
- Unit tests in ocelot use `include_bytes!` with embedded `modules.dep` fixtures from a real Linux kernel
