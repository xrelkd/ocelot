use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiGraph<L = String>
where
    L: Clone,
{
    adj: Vec<Vec<usize>>,
    rev_adj: Vec<Vec<usize>>,
    id_to_label: Vec<L>,
    label_to_id: HashMap<String, usize>,
}

impl<L> DiGraph<L>
where
    L: Clone,
{
    pub fn new() -> Self { Self::with_capacity(64) }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            adj: Vec::with_capacity(capacity),
            rev_adj: Vec::with_capacity(capacity),
            id_to_label: Vec::with_capacity(capacity),
            label_to_id: HashMap::with_capacity(capacity),
        }
    }

    pub fn add_node(&mut self, name: &str, label: L) -> usize {
        let id = self.id_to_label.len();
        let _ = self.label_to_id.insert(name.to_string(), id);
        self.id_to_label.push(label);
        self.adj.push(Vec::new());
        self.rev_adj.push(Vec::new());
        id
    }

    pub fn add_edge(&mut self, from: &str, to: &str) {
        if let (Some(&from_id), Some(&to_id)) =
            (self.label_to_id.get(from), self.label_to_id.get(to))
        {
            self.adj[from_id].push(to_id);
            self.rev_adj[to_id].push(from_id);
        }
    }

    pub fn get_id(&self, name: &str) -> Option<usize> { self.label_to_id.get(name).copied() }

    pub fn get_label(&self, id: usize) -> Option<&L> { self.id_to_label.get(id) }

    pub const fn node_count(&self) -> usize { self.id_to_label.len() }

    pub fn has_edge(&self, from: usize, to: usize) -> bool {
        self.adj.get(from).is_some_and(|neighbors| neighbors.contains(&to))
    }

    pub fn neighbors(&self, node: usize) -> &[usize] {
        self.adj.get(node).map_or(&[], |v| v.as_slice())
    }

    pub fn kosaraju_scc(&self) -> Vec<Vec<usize>> {
        let n = self.node_count();
        if n == 0 {
            return Vec::new();
        }

        let finish_order = {
            let mut finish_order = Vec::with_capacity(n);
            let mut visited = vec![false; n];

            for start in 0..n {
                if !visited[start] {
                    self.dfs_fill_order(start, &mut visited, &mut finish_order);
                }
            }
            finish_order
        };

        let mut sccs = Vec::new();
        let mut visited = vec![false; n];

        for &node in finish_order.iter().rev() {
            if !visited[node] {
                let mut component = Vec::new();
                self.dfs_rev(node, &mut visited, &mut component);
                sccs.push(component);
            }
        }

        sccs
    }

    fn dfs_fill_order(&self, start: usize, visited: &mut [bool], finish_order: &mut Vec<usize>) {
        let mut stack = vec![(start, 0)];
        visited[start] = true;

        while let Some((node, idx)) = stack.pop() {
            if idx < self.adj[node].len() {
                let next = self.adj[node][idx];
                stack.push((node, idx + 1));
                if !visited[next] {
                    visited[next] = true;
                    stack.push((next, 0));
                }
            } else {
                finish_order.push(node);
            }
        }
    }

    fn dfs_rev(&self, start: usize, visited: &mut [bool], component: &mut Vec<usize>) {
        let mut stack = vec![start];
        visited[start] = true;

        while let Some(node) = stack.pop() {
            component.push(node);
            for &next in &self.rev_adj[node] {
                if !visited[next] {
                    visited[next] = true;
                    stack.push(next);
                }
            }
        }
    }

    pub fn find_cycle_in_scc(&self, scc: &[usize], start: usize) -> Option<Vec<L>> {
        if scc.is_empty() {
            return None;
        }

        let scc_set: HashSet<_> = scc.iter().copied().collect();
        let mut stack = Vec::new();
        let mut on_stack = HashSet::new();
        let mut visited = HashSet::new();

        stack.push(start);
        let _ = on_stack.insert(start);
        let _ = visited.insert(start);

        while let Some(&node) = stack.last() {
            let mut found_next = false;

            for &neighbor in &self.adj[node] {
                if !scc_set.contains(&neighbor) {
                    continue;
                }

                if neighbor == node {
                    let cycle =
                        vec![self.id_to_label[node].clone(), self.id_to_label[node].clone()];
                    return Some(cycle);
                }

                if visited.insert(neighbor) {
                    stack.push(neighbor);
                    let _ = on_stack.insert(neighbor);
                    found_next = true;
                } else if on_stack.contains(&neighbor) {
                    let mut cycle = Vec::new();
                    for &idx in stack.iter().rev() {
                        cycle.push(self.id_to_label[idx].clone());
                        if idx == neighbor {
                            break;
                        }
                    }
                    cycle.reverse();
                    cycle.push(self.id_to_label[neighbor].clone());
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

    pub fn detect_cycle(&self) -> Option<Vec<L>> {
        let sccs = self.kosaraju_scc();

        for scc in sccs {
            if scc.len() > 1 {
                if let Some(&start) = scc.first()
                    && let Some(cycle) = self.find_cycle_in_scc(&scc, start)
                {
                    return Some(cycle);
                }
            } else if let Some(&node) = scc.first()
                && self.has_edge(node, node)
            {
                let label = self.id_to_label[node].clone();
                return Some(vec![label.clone(), label]);
            }
        }

        None
    }

    pub fn topological_order(&self) -> Vec<L> {
        let sccs = self.kosaraju_scc();
        let mut result = Vec::with_capacity(self.node_count());

        // Kosaraju returns SCCs in finish_order reverse, which gives us
        // SCCs in topological order but reversed. For A→B→C, we get [C, B, A].
        // Reverse to get proper order: A→B→C.
        for scc in sccs.into_iter().rev() {
            for node_id in scc {
                if let Some(label) = self.get_label(node_id) {
                    result.push(label.clone());
                }
            }
        }

        result
    }
}

impl<L> Default for DiGraph<L>
where
    L: Clone,
{
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::DiGraph;

    fn build_graph(labels: &[&str], edges: &[(usize, usize)]) -> DiGraph<String> {
        let mut graph = DiGraph::new();
        for label in labels {
            let _ = graph.add_node(label, label.to_string());
        }
        for (from, to) in edges {
            let from_name = labels[*from];
            let to_name = labels[*to];
            graph.add_edge(from_name, to_name);
        }
        graph
    }

    #[test]
    fn test_empty_graph() {
        let graph: DiGraph<String> = DiGraph::new();
        assert_eq!(graph.node_count(), 0);
        assert!(graph.kosaraju_scc().is_empty());
    }

    #[test]
    fn test_single_node_no_edges() {
        let graph = build_graph(&["A"], &[]);
        assert_eq!(graph.node_count(), 1);
        let sccs = graph.kosaraju_scc();
        assert_eq!(sccs.len(), 1);
        assert_eq!(sccs[0], vec![0]);
    }

    #[test]
    fn test_multiple_independent_nodes() {
        let graph = build_graph(&["A", "B", "C"], &[]);
        let sccs = graph.kosaraju_scc();
        assert_eq!(sccs.len(), 3);
        for scc in &sccs {
            assert_eq!(scc.len(), 1);
        }
    }

    #[test]
    fn test_simple_dag() {
        let graph = build_graph(&["A", "B", "C"], &[(0, 1), (1, 2)]);
        let sccs = graph.kosaraju_scc();
        assert_eq!(sccs.len(), 3);
        for scc in &sccs {
            assert_eq!(scc.len(), 1);
        }
    }

    #[test]
    fn test_two_node_cycle() {
        let graph = build_graph(&["A", "B"], &[(0, 1), (1, 0)]);
        let sccs = graph.kosaraju_scc();
        assert_eq!(sccs.len(), 1);
        assert_eq!(sccs[0].len(), 2);

        let cycle = graph.find_cycle_in_scc(&sccs[0], sccs[0][0]);
        assert!(cycle.is_some());
        let cycle = cycle.unwrap();
        assert_eq!(cycle.first(), cycle.last());
    }

    #[test]
    fn test_three_node_cycle() {
        let graph = build_graph(&["A", "B", "C"], &[(0, 1), (1, 2), (2, 0)]);
        let sccs = graph.kosaraju_scc();
        assert_eq!(sccs.len(), 1);
        assert_eq!(sccs[0].len(), 3);

        let cycle = graph.find_cycle_in_scc(&sccs[0], 0);
        assert!(cycle.is_some());
        let cycle = cycle.unwrap();
        assert_eq!(cycle.first(), cycle.last());
        assert!(cycle.len() >= 3);
    }

    #[test]
    fn test_self_loop() {
        let graph = build_graph(&["A"], &[(0, 0)]);
        let sccs = graph.kosaraju_scc();
        assert_eq!(sccs.len(), 1);

        let cycle = graph.find_cycle_in_scc(&sccs[0], 0);
        assert!(cycle.is_some());
        let cycle = cycle.unwrap();
        assert_eq!(cycle.len(), 2);
    }

    #[test]
    fn test_no_cycle_in_single_node_scc() {
        let graph = build_graph(&["A", "B", "C"], &[(0, 1), (1, 2)]);
        let sccs = graph.kosaraju_scc();
        for scc in &sccs {
            let cycle = graph.find_cycle_in_scc(scc, scc[0]);
            assert!(cycle.is_none());
        }
    }

    #[test]
    fn test_detect_cycle_in_dag() {
        let graph = build_graph(&["A", "B", "C"], &[(0, 1), (1, 2)]);
        let cycle = graph.detect_cycle();
        assert!(cycle.is_none());
    }

    #[test]
    fn test_detect_cycle_with_cycle() {
        let graph = build_graph(&["A", "B", "C"], &[(0, 1), (1, 2), (2, 0)]);
        let cycle = graph.detect_cycle();
        assert!(cycle.is_some());
    }

    #[test]
    fn test_topological_order() {
        let graph = build_graph(&["A", "B", "C"], &[(0, 1), (1, 2)]);
        let order = graph.topological_order();
        assert_eq!(order.len(), 3);
        let a_pos = order.iter().position(|l| l == "A").unwrap();
        let b_pos = order.iter().position(|l| l == "B").unwrap();
        let c_pos = order.iter().position(|l| l == "C").unwrap();
        assert!(c_pos < b_pos);
        assert!(b_pos < a_pos);
    }

    #[test]
    fn test_get_id_and_label() {
        let mut graph: DiGraph<String> = DiGraph::new();
        let id = graph.add_node("foo", "bar".to_string());
        assert_eq!(graph.get_id("foo"), Some(id));
        assert_eq!(graph.get_label(id), Some(&"bar".to_string()));
        assert_eq!(graph.get_id("nonexistent"), None);
        assert_eq!(graph.get_label(999), None);
    }

    #[test]
    fn test_add_edge_ignores_nonexistent() {
        let mut graph = build_graph(&["A", "B"], &[]);
        graph.add_edge("nonexistent", "A");
        graph.add_edge("A", "nonexistent");
        assert_eq!(graph.neighbors(0).len(), 0);
    }

    #[test]
    fn test_complex_scc() {
        let graph = build_graph(&["A", "B", "C", "D"], &[(0, 1), (1, 2), (2, 0), (2, 3), (3, 2)]);
        let sccs = graph.kosaraju_scc();
        assert_eq!(sccs.len(), 1);
        assert_eq!(sccs[0].len(), 4);
    }

    #[test]
    fn test_cycle_not_starting_from_first_node() {
        let graph = build_graph(&["A", "B", "C"], &[(0, 1), (1, 2), (2, 1)]);
        let sccs = graph.kosaraju_scc();
        assert!(!sccs.is_empty());

        let scc_with_bc = sccs.iter().find(|scc| scc.contains(&1) && scc.contains(&2));
        assert!(scc_with_bc.is_some());
        let scc = scc_with_bc.unwrap();

        let cycle = graph.find_cycle_in_scc(scc, 1);
        assert!(cycle.is_some());
    }

    #[test]
    fn test_four_node_cycle() {
        let graph = build_graph(&["A", "B", "C", "D"], &[(0, 1), (1, 2), (2, 3), (3, 0)]);
        let sccs = graph.kosaraju_scc();
        assert_eq!(sccs.len(), 1);

        let cycle = graph.find_cycle_in_scc(&sccs[0], 0);
        assert!(cycle.is_some());
        let cycle = cycle.unwrap();
        assert_eq!(cycle.first(), cycle.last());
        assert_eq!(cycle.len(), 5);
    }

    #[test]
    fn test_diamond_dag() {
        let graph = build_graph(&["A", "B", "C", "D"], &[(0, 1), (0, 2), (1, 3), (2, 3)]);
        let sccs = graph.kosaraju_scc();
        for scc in &sccs {
            assert_eq!(scc.len(), 1);
        }
        let order = graph.topological_order();
        let a_pos = order.iter().position(|l| l == "A").unwrap();
        let d_pos = order.iter().position(|l| l == "D").unwrap();
        assert!(d_pos < a_pos);
    }

    #[test]
    fn test_shared_dependency() {
        let graph = build_graph(&["A", "B", "C"], &[(0, 2), (1, 2)]);
        let order = graph.topological_order();
        let c_pos = order.iter().position(|l| l == "C").unwrap();
        let a_pos = order.iter().position(|l| l == "A").unwrap();
        let b_pos = order.iter().position(|l| l == "B").unwrap();
        assert!(c_pos < a_pos);
        assert!(c_pos < b_pos);
    }

    #[test]
    fn test_large_graph_128_nodes() {
        let mut graph = DiGraph::<String>::new();

        for i in 0..128 {
            let _ = graph.add_node(&format!("P_{i}"), format!("P_{i}"));
        }

        // Linear chain: P_0 → P_1 → ... → P_127
        for i in 0..127 {
            graph.add_edge(&format!("P_{i}"), &format!("P_{}", i + 1));
        }

        assert_eq!(graph.node_count(), 128);
        let sccs = graph.kosaraju_scc();
        assert!(sccs.iter().all(|scc| scc.len() == 1));

        let order = graph.topological_order();
        assert_eq!(order.len(), 128);

        let cycle = graph.detect_cycle();
        assert!(cycle.is_none());
    }

    #[test]
    fn test_large_graph_512_nodes_with_cycle() {
        let mut graph = DiGraph::<String>::new();

        for i in 0..512 {
            let _ = graph.add_node(&format!("P_{i}"), format!("P_{i}"));
        }

        // Create a cycle by wrapping around
        for i in 0..512 {
            let deps = [(i + 1) % 512, (i + 2) % 512, (i + 3) % 512];
            for dep in deps {
                graph.add_edge(&format!("P_{i}"), &format!("P_{dep}"));
            }
        }

        assert_eq!(graph.node_count(), 512);
        let _sccs = graph.kosaraju_scc();
        let cycle = graph.detect_cycle();
        assert!(cycle.is_some());
    }

    #[test]
    fn test_large_graph_1024_nodes_with_cycle() {
        let mut graph = DiGraph::<String>::new();

        for i in 0..1024 {
            let _ = graph.add_node(&format!("P_{i}"), format!("P_{i}"));
        }

        // Create a cycle by wrapping around
        for i in 0..1024 {
            let deps = [(i + 1) % 1024, (i + 2) % 1024, (i + 3) % 1024];
            for dep in deps {
                graph.add_edge(&format!("P_{i}"), &format!("P_{dep}"));
            }
        }

        assert_eq!(graph.node_count(), 1024);
        let cycle = graph.detect_cycle();
        assert!(cycle.is_some());

        let order = graph.topological_order();
        assert!(order.len() <= 1024);
    }
}
