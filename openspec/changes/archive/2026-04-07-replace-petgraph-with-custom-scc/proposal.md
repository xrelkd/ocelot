## Why

Ocelot currently depends on `petgraph` for dependency graph processing (topological sort, cycle detection). This dependency adds overhead for a relatively simple use case — Ocelot's process and module dependency graphs typically have 10-500 nodes. A custom SCC-based implementation would eliminate an external dependency and give us full control over the algorithm.

## What Changes

- Create new `ocelot/src/graph/` module with a custom `DiGraph<L>` struct using coordinate compression
- Implement Kosaraju's SCC algorithm (iterative, no recursion) for cycle detection
- Replace petgraph usage in 4 config files:
  - `ocelot/src/config/utils.rs`
  - `ocelot/src/config/supervise/mod.rs`
  - `ocelot/src/config/bootstrap/supervise/bootstrap.rs`
  - `ocelot/src/config/bootstrap/modules/mod.rs`
- Add comprehensive tests for the new graph module
- Remove `petgraph` from workspace dependencies

## Capabilities

### New Capabilities

- `custom-graph-scc`: Custom directed graph implementation with Kosaraju's SCC algorithm for dependency cycle detection

### Modified Capabilities

- None — this is an internal implementation change with no spec-level behavior changes

## Impact

- **Code**: New `ocelot/src/graph/mod.rs` module; updates to config validation modules
- **Dependencies**: Remove `petgraph` from `Cargo.toml`
- **APIs**: No external API changes — internal implementations only
