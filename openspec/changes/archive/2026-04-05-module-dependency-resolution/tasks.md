## 1. Ocelot Serde Config Changes

- [x] 1.1 Add `dep_file_path: Option<String>` to `ModulesConfig::List` in `ocelot/src/config/bootstrap.rs` with serde attributes
- [x] 1.2 Add `dep_file_path: String` (required) and `names: Option<Vec<String>>` to `ModulesConfig::Scan` in `ocelot/src/config/bootstrap.rs` with serde attributes

## 2. Error Types (ocelot)

- [x] 2.1 Add `ParseModuleDependencyFile { path: String, source: std::io::Error }` error variant to ocelot config error
- [x] 2.2 Add `ModuleNotFound { name: String }` error variant to ocelot validation error (reuse existing `CyclicDependency` from `ValidationError`)

## 3. Dependency File Parser (ocelot)

- [x] 3.1 Implement `parse_dep_file(data: &[u8]) -> HashMap<String, Vec<String>>` that parses `modules.dep` text format
- [x] 3.2 Handle `.ko`, `.ko.xz`, `.ko.gz` extensions in parsing
- [x] 3.3 Write unit tests using `include_bytes!`: empty file, no deps, with deps, mixed entries

## 4. Topological Sort & Cycle Detection (ocelot, reuse petgraph)

- [x] 4.1 Implement `resolve_module_order(dep_map, targets)` using `petgraph::algo::toposort`
- [x] 4.2 Reuse `petgraph::algo::kosaraju_scc` + DFS cycle extraction (existing `find_cycle_in_scc` in utils)
- [x] 4.3 Write unit tests using `include_bytes!` fixtures (all in ocelot config tests):
  - Happy path: `modules-virtio.dep` (real Linux kernel virtio subset) — verify virtio_net loads after virtio_ring, virtio, net_failover, failover
  - 2-node cycle: `modules-cycle.dep` — virtio_net ↔ virtio
  - 3-node cycle: `modules-cycle3.dep` — a → b → c → a
  - Self-loop: `modules-selfloop.dep` — a → a
  - Empty file: `modules-empty.dep`

## 5. Config Validation Integration (ocelot)

- [x] 5.1 Add `validate_module_dependencies()` to `BootstrapConfig::validate()` — parses depfile, resolves order, detects cycles
- [x] 5.2 On success: replace `names` with topologically sorted order
- [x] 5.3 On cycle: return `ValidationError::CyclicDependency` with full cycle path
- [x] 5.4 On missing dep: return `ValidationError::ModuleNotFound`
- [x] 5.5 Update `impl From<ModulesConfig> for ocelot_bootstrap::ModulesConfig` to pass sorted `names` (no `dep_file_path` forwarded)

## 6. Bootstrap Documentation

- [x] 6.1 Add doc comments to `ModulesConfig` in `crates/bootstrap/src/config.rs` clarifying that `names` are assumed to be in correct dependency order as validated by the ocelot config layer

## 7. Test Data

- [x] 7.1 Create `ocelot/src/config/test_data/` directory with `.dep` fixtures:
  - `modules-virtio.dep` — real kernel virtio subsystem deps (happy path)
  - `modules-cycle.dep` — 2-node cycle
  - `modules-cycle3.dep` — 3-node cycle
  - `modules-selfloop.dep` — self-loop
  - `modules-empty.dep` — empty file

## 8. Verification

- [x] 8.1 Run `cargo fmt --all --check`, `cargo clippy-all`, `cargo nextest run`
