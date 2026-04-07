## ADDED Requirements

### Requirement: DiGraph supports generic label type

The graph implementation SHALL support generic label type `L` with `Clone` bound, defaulting to `String`.

#### Scenario: Default String label

- **WHEN** a `DiGraph` is created without specifying label type
- **THEN** the graph uses `String` as the default label type

#### Scenario: Custom label type

- **WHEN** a `DiGraph<String>` is created
- **THEN** the graph stores and returns `String` labels

### Requirement: DiGraph supports coordinate compression

The graph SHALL use coordinate compression for node mapping, with O(1) lookup by name.

#### Scenario: Add node by name

- **WHEN** `add_node("foo", label)` is called
- **THEN** the node is added and assigned a unique numeric ID

#### Scenario: Lookup node ID

- **WHEN** `get_id("foo")` is called
- **THEN** returns `Some(usize)` if node exists, `None` otherwise

#### Scenario: Lookup node label

- **WHEN** `get_label(0)` is called
- **THEN** returns `Some(&L)` if ID is valid, `None` otherwise

### Requirement: DiGraph supports edge operations

The graph SHALL support adding directed edges between nodes by name.

#### Scenario: Add edge between existing nodes

- **WHEN** nodes "A" and "B" exist and `add_edge("A", "B")` is called
- **THEN** a directed edge from A to B is created

#### Scenario: Add edge with non-existent nodes

- **WHEN** `add_edge("nonexistent", "A")` is called
- **THEN** the operation is silently ignored (no panic)

### Requirement: Kosaraju SCC algorithm

The graph SHALL implement Kosaraju's algorithm to find all strongly connected components.

#### Scenario: Single node SCC

- **WHEN** graph has nodes A with no edges
- **THEN** `kosaraju_scc()` returns `[[0]]` (one SCC with one node)

#### Scenario: Multiple independent nodes

- **WHEN** graph has nodes A, B, C with no edges between them
- **THEN** `kosaraju_scc()` returns `[[0], [1], [2]]` (three separate SCCs)

#### Scenario: SCC with cycle

- **WHEN** graph has edges A→B, B→C, C→A
- **THEN** `kosaraju_scc()` returns `[[0, 1, 2]]` (one SCC with all three nodes)

#### Scenario: Self-loop detected as SCC

- **WHEN** graph has node A with edge A→A
- **THEN** `kosaraju_scc()` returns `[[0]]` (single node SCC)

### Requirement: Cycle extraction in SCC

The graph SHALL support extracting a cycle path from a strongly connected component, returning labels for error display.

#### Scenario: Find cycle in two-node SCC

- **WHEN** SCC contains nodes [0, 1] with edges forming a cycle
- **THEN** `find_cycle_in_scc(&[0, 1], 0)` returns `Some(["A", "B", "A"])` (cycle path with labels)

#### Scenario: No cycle in single-node SCC

- **WHEN** SCC contains single node [0] with no self-loop
- **THEN** `find_cycle_in_scc(&[0], 0)` returns `None`

#### Scenario: Cycle in three-node SCC

- **WHEN** SCC contains nodes [0, 1, 2] with edges forming a cycle
- **THEN** `find_cycle_in_scc(&[0, 1, 2], 0)` returns a cycle path with labels where first == last

### Requirement: Cycle detection integration

The graph SHALL provide cycle detection by analyzing SCC results.

#### Scenario: Detect cycle in DAG

- **WHEN** graph is a DAG with edges A→B→C
- **THEN** all SCCs have size 1, no cycles detected

#### Scenario: Detect cycle in graph with cycle

- **WHEN** graph has edges A→B, B→C, C→A
- **THEN** one SCC has size > 1, cycle is detected

### Requirement: Backwards compatible error format

The cycle detection SHALL produce error messages compatible with existing petgraph format.

#### Scenario: Cycle error message format

- **WHEN** cycle is detected in nodes ["A", "B", "C"]
- **THEN** error message contains "A → B → C → A" format

### Requirement: Graph properties

The graph SHALL provide methods to query its state.

#### Scenario: Node count

- **WHEN** graph has 3 nodes added
- **THEN** `node_count()` returns 3

#### Scenario: Empty graph

- **WHEN** graph is newly created
- **THEN** `node_count()` returns 0 and `kosaraju_scc()` returns empty vec
