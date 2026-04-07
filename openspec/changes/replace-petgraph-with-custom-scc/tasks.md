## 1. Create Graph Module

- [ ] 1.1 Create `ocelot/src/graph/mod.rs` with `DiGraph<L>` struct
- [ ] 1.2 Implement coordinate compression: `label_to_id` HashMap and `id_to_label` Vec
- [ ] 1.3 Implement `add_node(name, label)` and `add_edge(from, to)` methods
- [ ] 1.4 Implement `get_id(name)` and `get_label(id)` lookup methods
- [ ] 1.5 Implement `node_count()` and other accessor methods

## 2. Implement Kosaraju SCC Algorithm

- [ ] 2.1 Implement iterative DFS for fill order (first pass)
- [ ] 2.2 Implement iterative DFS on reversed graph (second pass)
- [ ] 2.3 Implement `kosaraju_scc()` returning `Vec<Vec<usize>>`
- [ ] 2.4 Ensure SCCs are in reverse topological order

## 3. Implement Cycle Extraction

- [ ] 3.1 Reimplement `find_cycle_in_scc(scc, start)` returning labels (Vec<L>) for error messages
- [ ] 3.2 Handle edge cases: single node SCC, self-loops, multi-node cycles

## 4. Add Comprehensive Tests

- [ ] 4.1 Add tests for DiGraph basic operations (add node, add edge, lookup)
- [ ] 4.2 Add tests for Kosaraju SCC: single nodes, multiple SCCs, cycles
- [ ] 4.3 Add tests for cycle extraction: two-node cycle, three-node cycle, self-loop
- [ ] 4.4 Add tests for DAG (no cycles) verification
- [ ] 4.5 Add tests for empty graph edge cases

## 5. Refactor config/supervise/mod.rs

- [ ] 5.1 Remove petgraph imports and replace with `crate::graph::DiGraph`
- [ ] 5.2 Update `detect_dependency_cycles()` to use new graph and SCC
- [ ] 5.3 Preserve error message format for backwards compatibility
- [ ] 5.4 Run existing tests to verify no regressions

## 6. Refactor config/bootstrap/supervise/bootstrap.rs

- [ ] 6.1 Remove petgraph imports and replace with `crate::graph::DiGraph`
- [ ] 6.2 Update `detect_dependency_cycles()` to use new graph and SCC
- [ ] 6.3 Preserve error message format for backwards compatibility
- [ ] 6.4 Run existing tests to verify no regressions

## 7. Refactor config/bootstrap/modules/mod.rs

- [ ] 7.1 Remove petgraph imports and replace with `crate::graph::DiGraph`
- [ ] 7.2 Update `resolve_module_order()` to use new graph and SCC
- [ ] 7.3 Preserve error message format for backwards compatibility
- [ ] 7.4 Run existing tests to verify no regressions

## 8. Update config/utils.rs

- [ ] 8.1 Remove petgraph imports from utils.rs
- [ ] 8.2 Update `find_cycle_in_scc` to work with new graph structure
- [ ] 8.3 Ensure utils tests still pass

## 9. Remove petgraph Dependency

- [ ] 9.1 Remove `petgraph` from workspace dependencies in `Cargo.toml`
- [ ] 9.2 Verify no other crates in workspace use petgraph
- [ ] 9.3 Run full test suite to ensure everything works

## 10. Final Verification

- [ ] 10.1 Run `cargo build` to verify compilation
- [ ] 10.2 Run `cargo test` to verify all tests pass
- [ ] 10.3 Run `cargo clippy-all` to check for lint issues
- [ ] 10.4 Review that all specs requirements are covered by implementation
