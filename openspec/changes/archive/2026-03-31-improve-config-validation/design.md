## Context

Ocelot is a process supervisor written in Rust. The current configuration validation logic resides in `SupervisorConfig::validate()` in `ocelot/src/config/mod.rs`. It checks three things: version compatibility, missing dependencies, and cyclic dependencies. Validation runs automatically when starting the supervisor but there's no standalone CLI command to validate a configuration file without starting the supervisor. Additionally, many configuration fields lack explicit validation, including log rotation parameters, probe timeouts, restart backoff durations, and port numbers. This results in runtime errors that could be caught earlier with clearer feedback.

## Goals / Non-Goals

**Goals:**

- Provide a CLI subcommand `ocelot supervise validate <config-file>` for standalone configuration validation with proper exit codes (0 for valid, 1 for invalid).
- Extend `SupervisorConfig::validate()` to include comprehensive checks on all configuration fields.
- Deliver clear, actionable error messages that point directly to the misconfiguration.
- Keep all changes backward compatible; existing valid configurations must continue to work.

**Non-Goals:**

- Changing the configuration schema or serialization format.
- Validating external runtime conditions (e.g., whether the program executable exists, network port availability).
- Supporting legacy configuration versions beyond the current `1.0`.
- Changing the behavior of the `Run` subcommand beyond additional validation.

## Decisions

### Extend `SupervisorConfig::validate()`

- **Decision**: Add new private helper methods to `SupervisorConfig` that iterate over processes and validate individual fields, called from `validate()`.
- **Alternatives**: Create a separate `Validator` struct. **Rejected** because `validate()` is already a method; keeping logic in the same module maintains cohesion and allows reuse during `Run`.

### CLI Subcommand Design

- **Decision**: Add `Validate` variant to `supervise::Commands` enum. The handler loads the config, runs `validate()`, prints success or error, and exits with appropriate code.
- **Alternatives**: Make `validate` a top-level command. **Rejected** because it's clearly related to supervision configuration and fits under the `supervise` umbrella.

### Output Format

- **Decision**: Support both human-readable and machine-readable (JSON) output. The default is human-readable to stderr for errors and stdout for success. Add an `--output` flag with options `human` (default) and `json`.
- **Alternatives**: Human-only initially. **Rejected** because machine-readable output is needed for automation and tooling integration.
- **Implementation Note**: The `validate` subcommand SHALL use `println!` for success messages to stdout and `eprintln!` for error messages to stderr. Do not use `tracing` for this command to avoid log level interference and ensure clean, controlled output.

### Error Handling

- **Decision**: Add new error variants to `crate::error::Error` (e.g., `InvalidLogRotation`, `InvalidProbe`, `InvalidRestartPolicy`) and use `snafu` for context.
- **Alternatives**: Use `validation_errors` vector to collect all errors at once. **Deferred**: Initially report first error for simplicity; can batch errors later if needed.

### Validation Scope

- **Decision**: Implement the following checks:
  - Log rotation: if any rotation parameter is set, `max_size_bytes > 0`, `rotation_interval_secs > 0`, `max_files > 0`, `max_age_days > 0`. Additionally, when rotation is enabled, both `rotation_interval_secs` and `max_size_bytes` cannot both be zero (would never rotate).
  - Probes: `timeout <= period`; for HTTP probes, `port` in 1-65535; for TCP, same port check; `initial_delay` non-negative.
  - Restart: `backoff` duration > 0 if present.
  - Process: `program` not empty; `termination_grace_period` > 0.
  - Environment variables: Detect duplicate keys in YAML and report an error (since `HashMap` silently overwrites).
  - Dependency cycles: Enhance the topological sort to extract and report the full cycle path (e.g., "A → B → C → A") instead of just the single node.
- **Alternatives**: Enforce stricter checks like absolute program paths. **Rejected** to avoid breaking container use-cases where binaries may be relative to working directory.

## Risks / Trade-offs

- **Risk**: New validation may reject configurations that previously started but failed later (e.g., negative durations). This could break existing deployments with invalid configs.
  - **Mitigation**: Document the change clearly; provide explicit error messages with suggested fixes.

- **Risk**: Performance overhead from additional validation on every `Run`.
  - **Mitigation**: Validation is O(n) in number of processes; checks are simple comparisons. Overhead is negligible compared to process startup.

- **Risk**: Adding many error variants could increase enum size and require updates to error conversion logic.
  - **Mitigation**: Follow existing patterns; add tests alongside.

- **Risk**: Enforcing duplicate environment variable detection may fail for configs that previously worked (serde's last-wins).
  - **Mitigation**: This is intentional to avoid silently dropped environment variables; users must fix duplicates.

- **Risk**: Cycle detection algorithm enhancement (SCC + DFS) adds complexity.
  - **Mitigation**: Keep the implementation straightforward: use petgraph's `kosaraju_scc` to identify the strongly connected component containing the failing node, then perform a simple DFS within that SCC to extract one cycle path.

## Migration Plan

No migration required. The change is fully backward compatible for valid configurations. Invalid configurations that previously succeeded will now fail early with clear error messages pointing to the exact problem. Users should update their configs based on the validation output. The `validate` subcommand can be used to test configs before deploying.

## Implementation Details: Cycle Detection Algorithm

To produce a full cycle path (e.g., "A → B → C → A") rather than just a single node:

1. When `petgraph::algo::toposort(&graph, None)` returns `Err(cycle)`, extract the failing node: `let node = cycle.node_id()`.
2. Run `let sccs = petgraph::algo::kosaraju_scc(&graph)` to get all strongly connected components.
3. Find the SCC that contains `node`. If its size > 1, it contains a cycle.
4. Perform a depth-first search (DFS) within that SCC starting from `node`, tracking parent pointers and a recursion stack (set of visited nodes in current DFS path). When we encounter a node already in the stack, we've found a back edge and can reconstruct the cycle by walking back from the current node to the repeated node using parent pointers.
5. Format the cycle as a list of node names (graph node values) in traversal order.

This approach uses petgraph's SCC algorithm to limit the search space, making it efficient even for larger graphs.

## Open Questions

Resolved:

- Duplicate environment variables: validated and rejected.
- Output format: both human (default) and JSON via `--output json`.
- Rotation both zero: treated as invalid when rotation is configured.
- Cycle detection: use kosaraju_scc + DFS to extract path.
