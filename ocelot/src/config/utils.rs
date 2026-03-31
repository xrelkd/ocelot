use std::collections::HashSet;

use petgraph::{Direction, graph::NodeIndex};

/// Find a cycle within the given strongly connected component starting from
/// `start`. Returns a list of node indices representing the cycle,
/// where the first and last nodes are the same (the cycle is closed).
pub fn find_cycle_in_scc(
    graph: &petgraph::Graph<String, ()>,
    scc: &[NodeIndex],
    start: NodeIndex,
) -> Option<Vec<NodeIndex>> {
    let scc_set: HashSet<_> = scc.iter().copied().collect();
    let mut stack = Vec::new();
    let mut on_stack = HashSet::new();
    let mut visited = HashSet::new();

    stack.push(start);
    let _ = on_stack.insert(start);
    let _ = visited.insert(start);

    while let Some(&node) = stack.last() {
        let mut found_next = false;
        for neighbor in graph.neighbors_directed(node, Direction::Outgoing) {
            if !scc_set.contains(&neighbor) {
                continue;
            }
            if visited.insert(neighbor) {
                stack.push(neighbor);
                let _ = on_stack.insert(neighbor);
                found_next = true;
            } else if on_stack.contains(&neighbor) {
                let mut cycle = Vec::new();
                for &idx in stack.iter().rev() {
                    cycle.push(idx);
                    if idx == neighbor {
                        break;
                    }
                }
                cycle.reverse();
                cycle.push(neighbor);
                return Some(cycle);
            }
        }
        if !found_next {
            let _ = stack.pop();
            let _ = on_stack.remove(&node);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use petgraph::Graph;

    use super::find_cycle_in_scc;

    fn build_graph(nodes: &[&str], edges: &[(usize, usize)]) -> Graph<String, ()> {
        let mut graph = Graph::<String, ()>::new();
        let node_indices: Vec<_> = nodes.iter().map(|n| graph.add_node(n.to_string())).collect();
        for &(from, to) in edges {
            let _ = graph.add_edge(node_indices[from], node_indices[to], ());
        }
        graph
    }

    #[test]
    fn test_simple_cycle_two_nodes() {
        let graph = build_graph(&["A", "B"], &[(0, 1), (1, 0)]);
        let scc: Vec<_> = graph.node_indices().collect();
        let cycle = find_cycle_in_scc(&graph, &scc, petgraph::graph::NodeIndex::new(0));
        assert!(cycle.is_some());
        let cycle = cycle.unwrap();
        assert!(cycle.first() == cycle.last());
        assert_eq!(cycle.len(), 3);
    }

    #[test]
    fn test_cycle_three_nodes() {
        let graph = build_graph(&["A", "B", "C"], &[(0, 1), (1, 2), (2, 0)]);
        let scc: Vec<_> = graph.node_indices().collect();
        let cycle = find_cycle_in_scc(&graph, &scc, petgraph::graph::NodeIndex::new(0));
        assert!(cycle.is_some());
        let cycle = cycle.unwrap();
        assert!(cycle.first() == cycle.last());
        assert_eq!(cycle.len(), 4);
    }

    #[test]
    fn test_no_cycle_single_node() {
        let graph = build_graph(&["A"], &[]);
        let scc: Vec<_> = graph.node_indices().collect();
        let cycle = find_cycle_in_scc(&graph, &scc, petgraph::graph::NodeIndex::new(0));
        assert!(cycle.is_none());
    }

    #[test]
    fn test_self_loop() {
        let graph = build_graph(&["A"], &[(0, 0)]);
        let scc: Vec<_> = graph.node_indices().collect();
        let cycle = find_cycle_in_scc(&graph, &scc, petgraph::graph::NodeIndex::new(0));
        assert!(cycle.is_some());
        let cycle = cycle.unwrap();
        assert_eq!(cycle.len(), 2);
        assert!(cycle.first() == cycle.last());
    }

    #[test]
    fn test_no_cycle_dag() {
        let graph = build_graph(&["A", "B", "C"], &[(0, 1), (1, 2)]);
        let scc: Vec<_> = graph.node_indices().collect();
        let cycle = find_cycle_in_scc(&graph, &scc, petgraph::graph::NodeIndex::new(0));
        assert!(cycle.is_none());
    }

    #[test]
    fn test_complex_scc() {
        let graph = build_graph(&["A", "B", "C", "D"], &[(0, 1), (1, 2), (2, 0), (2, 3), (3, 2)]);
        let scc: Vec<_> = graph.node_indices().collect();
        let cycle = find_cycle_in_scc(&graph, &scc, petgraph::graph::NodeIndex::new(0));
        assert!(cycle.is_some());
        let cycle = cycle.unwrap();
        assert!(cycle.first() == cycle.last());
        assert!(cycle.len() >= 3);
    }

    #[test]
    fn test_cycle_not_starting_from_first_node() {
        let graph = build_graph(&["A", "B", "C"], &[(0, 1), (1, 2), (2, 1)]);
        let scc: Vec<_> = graph.node_indices().collect();
        let cycle = find_cycle_in_scc(&graph, &scc, petgraph::graph::NodeIndex::new(1));
        assert!(cycle.is_some());
        let cycle = cycle.unwrap();
        assert!(cycle.first() == cycle.last());
    }

    #[test]
    fn test_four_node_cycle() {
        let graph = build_graph(&["A", "B", "C", "D"], &[(0, 1), (1, 2), (2, 3), (3, 0)]);
        let scc: Vec<_> = graph.node_indices().collect();
        let cycle = find_cycle_in_scc(&graph, &scc, petgraph::graph::NodeIndex::new(0));
        assert!(cycle.is_some());
        let cycle = cycle.unwrap();
        assert!(cycle.first() == cycle.last());
        assert_eq!(cycle.len(), 5);
    }
}
