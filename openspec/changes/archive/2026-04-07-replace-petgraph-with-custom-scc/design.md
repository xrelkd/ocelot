## Context

Ocelot currently uses `petgraph` for dependency graph processing in config validation. The dependency is used in:

- `ocelot/src/config/supervise/mod.rs` — process dependency cycle detection
- `ocelot/src/config/bootstrap/supervise/bootstrap.rs` — bootstrap supervise config validation
- `ocelot/src/config/bootstrap/modules/mod.rs` — kernel module dependency resolution
- `ocelot/src/config/utils.rs` — cycle extraction helper

Current implementation uses `petgraph::algo::toposort()` to detect cycles, and when a cycle is found, runs `petgraph::algo::kosaraju_scc()` to extract the strongly connected component containing the cycle, then uses `find_cycle_in_scc()` to get the full cycle path.

## Goals / Non-Goals

**Goals:**

- Replace petgraph with custom graph implementation using Kosaraju's SCC algorithm
- Use coordinate compression (String → usize ID mapping) for efficiency with 500+ nodes
- Support generic label type `L: Clone` for flexibility
- Maintain existing behavior and error messages for cycle detection
- Add comprehensive tests for the new graph module

**Non-Goals:**

- Support other graph algorithms (Dijkstra, A\*, etc.) — only need SCC/cycle detection
- Provide a public crate-level API — internal to ocelot binary only

## Decisions

### 1. Drop toposort, use Kosaraju SCC directly

**Decision**: Remove `toposort()` method entirely. Use Kosaraju's SCC algorithm as the primary algorithm — it already returns SCCs in reverse topological order.

**Rationale**: The current code uses toposort only to detect cycles. When it fails, it immediately runs Kosaraju to find the SCC. We can skip the toposort step entirely:

- Run Kosaraju's SCC once
- Any SCC with >1 node OR self-loop indicates a cycle
- SCCs are already in reverse topological order — no separate sorting needed

**Alternative considered**: Kahn's algorithm for topological sort — rejected because it doesn't provide cycle path extraction.

### 2. Use coordinate compression for node mapping

**Decision**: Store nodes with coordinate compression (String name → usize ID mapping).

```rust
pub struct DiGraph<L: Clone> {
    adj: Vec<Vec<usize>>,
    rev_adj: Vec<Vec<usize>>,
    id_to_label: Vec<L>,
    label_to_id: HashMap<String, usize>,
}
```

**Rationale**: With 500+ modules/processes expected:

- Reduces memory by storing labels once in `id_to_label` vector
- Enables O(1) node lookup by name via HashMap
- Simpler integer-based adjacency lists

**Alternative considered**: Direct label storage without compression — rejected because it duplicates labels across adjacency lists and doesn't scale as well.

### 3. Generic label type L with Clone bound

**Decision**: Use generic `L: Clone` for node labels, with `String` as the default.

```rust
pub struct DiGraph<L: Clone = String> {
    // ...
}
```

**Rationale**:

- Allows flexibility for future use cases (custom label types, IDs, etc.)
- `Clone` is sufficient — we don't need `Copy` (graph owns labels)
- Default to `String` for backwards compatibility with current usage

**Alternative considered**: Require `Eq + Hash` — not needed with coordinate compression since we use `String` as the key.

### 4. Name-based API for add_edge

**Decision**: Public API accepts `&str` names; internal ID-based for performance.

```rust
pub fn add_edge(&mut self, from: &str, to: &str)
```

**Rationale**: More ergonomic for callers — they don't need to track node IDs manually. Internally, we use the `label_to_id` map for lookup.

**Alternative considered**: ID-based API — rejected because it forces callers to manage ID mapping themselves.

### 5. Iterative implementation (no recursion)

**Decision**: Use iterative stack-based DFS for Kosaraju to avoid stack overflow on large graphs.

**Rationale**: Recursive DFS can overflow the stack with 500+ nodes. Iterative implementation using explicit stack is safer.

## Risks / Trade-offs

| Risk                   | Impact | Mitigation                                                                             |
| ---------------------- | ------ | -------------------------------------------------------------------------------------- |
| Algorithm correctness  | High   | Comprehensive tests covering: DAGs, cycles, self-loops, multi-node SCCs, edge cases    |
| Performance regression | Medium | Benchmark if needed; Ocelot's graph sizes are small enough that difference is minimal  |
| Missing edge cases     | Medium | Review existing petgraph usage patterns in tests; ensure error messages match original |
| Breaking error format  | Low    | Preserve cycle path format (e.g., `"A → B → C → A"`) for compatibility                 |

## Migration Plan

1. Create `ocelot/src/graph/mod.rs` with `DiGraph<L>` implementation
2. Update `ocelot/src/config/utils.rs` to use the new graph module (remove petgraph imports)
3. Refactor `ocelot/src/config/supervise/mod.rs` — replace petgraph with `crate::graph::DiGraph`
4. Refactor `ocelot/src/config/bootstrap/supervise/bootstrap.rs` — same pattern
5. Refactor `ocelot/src/config/bootstrap/modules/mod.rs` — same pattern
6. Add tests in `ocelot/src/graph/mod.rs`
7. Run existing tests to verify no regressions
8. Remove `petgraph` from workspace dependencies in `Cargo.toml`

## Open Questions

- Should `find_cycle_in_scc` return cycle nodes as IDs or labels? (Currently returns petgraph NodeIndex — we'll return labels for backwards compatibility with error messages)
- Should we preserve the specific error message format from petgraph? (Yes — for backwards compatibility)
