use std::collections::{HashMap, HashSet};

use petgraph::{Graph, algo, graph::NodeIndex};

use self::error::ValidationError;

/// Parse a `modules.dep` text file and return a mapping of module basenames to
/// their dependency basenames.
///
/// Each line has the format:
/// `kernel/path/to/module.ko.xz: kernel/path/to/dep1.ko.xz
/// kernel/path/to/dep2.ko.xz`
///
/// Returns a map from module basename (e.g., `virtio_net.ko.xz`) to a list of
/// dependency basenames.
pub fn parse_dep_file(data: &[u8]) -> HashMap<String, Vec<String>> {
    let text = String::from_utf8_lossy(data);
    let mut map = HashMap::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let Some((module_part, deps_part)) = line.split_once(':') else {
            continue;
        };

        let module_name = basename(module_part.trim());
        let deps: Vec<String> = deps_part.split_whitespace().map(basename).collect();

        let _prev = map.insert(module_name, deps);
    }

    map
}

/// Extract the basename (filename component) from a module path.
fn basename(path: &str) -> String { path.rsplit('/').next().unwrap_or(path).to_string() }

/// Resolve a valid module loading order from a dependency map and a list of
/// target modules.
///
/// Performs topological sort using petgraph. If a cycle is detected, returns
/// a `CyclicDependency` error with the full cycle path.
///
/// Only the targets and their transitive dependencies are included in the
/// result. Extra entries in the dependency map are ignored.
pub fn resolve_module_order(
    dep_map: &HashMap<String, Vec<String>>,
    targets: &[String],
) -> Result<Vec<String>, ValidationError> {
    let needed = collect_transitive_deps(dep_map, targets)?;

    let mut graph = Graph::<String, ()>::new();
    let mut indices: HashMap<String, NodeIndex> = HashMap::new();

    for name in &needed {
        let _ = indices.insert(name.clone(), graph.add_node(name.clone()));
    }

    for name in &needed {
        let from = indices[name];
        if let Some(deps) = dep_map.get(name) {
            for dep in deps {
                if needed.contains(dep) {
                    let to = indices[dep];
                    let _ = graph.add_edge(from, to, ());
                }
            }
        }
    }

    match algo::toposort(&graph, None) {
        Ok(sorted) => {
            // toposort returns dependents first; we need dependencies first.
            let mut order: Vec<String> = sorted.into_iter().map(|idx| graph[idx].clone()).collect();
            order.reverse();
            Ok(order)
        }
        Err(cycle_err) => {
            let node = cycle_err.node_id();
            let sccs = algo::kosaraju_scc(&graph);
            let scc = sccs.iter().find(|scc| scc.contains(&node)).cloned();

            let Some(scc) = scc else {
                return Err(ValidationError::CyclicDependency { cycle: vec![graph[node].clone()] });
            };

            super::utils::find_cycle_in_scc(&graph, &scc, node).map_or_else(
                || Err(ValidationError::CyclicDependency { cycle: vec![graph[node].clone()] }),
                |cycle_nodes| {
                    let cycle: Vec<String> =
                        cycle_nodes.into_iter().map(|idx| graph[idx].clone()).collect();
                    Err(ValidationError::CyclicDependency { cycle })
                },
            )
        }
    }
}

/// Collect all transitive dependencies for a set of target modules.
///
/// Returns the full set of module basenames needed (targets + all deps).
fn collect_transitive_deps(
    dep_map: &HashMap<String, Vec<String>>,
    targets: &[String],
) -> Result<HashSet<String>, ValidationError> {
    let mut needed = HashSet::new();
    let mut stack: Vec<&str> = targets.iter().map(String::as_str).collect();

    while let Some(name) = stack.pop() {
        if !needed.insert(name.to_string()) {
            continue;
        }

        let Some(deps) = dep_map.get(name) else {
            return Err(ValidationError::ModuleNotFound { name: name.to_string() });
        };

        for dep in deps {
            if !needed.contains(dep) {
                stack.push(dep);
            }
        }
    }

    Ok(needed)
}

#[cfg(test)]
mod tests {
    use super::{parse_dep_file, resolve_module_order};

    const VIRTIO_DEP: &[u8] = include_bytes!("test_data/modules-virtio.dep");
    const CYCLE_DEP: &[u8] = include_bytes!("test_data/modules-cycle.dep");
    const CYCLE3_DEP: &[u8] = include_bytes!("test_data/modules-cycle3.dep");
    const SELFLOOP_DEP: &[u8] = include_bytes!("test_data/modules-selfloop.dep");

    #[test]
    fn test_parse_empty_depfile() {
        let map = parse_dep_file(&[]);
        assert!(map.is_empty());
    }

    #[test]
    fn test_parse_depfile_no_deps() {
        let data = b"kernel/foo.ko.xz:\n";
        let map = parse_dep_file(data);
        let empty: Vec<String> = vec![];
        assert_eq!(map.get("foo.ko.xz"), Some(&empty));
    }

    #[test]
    fn test_parse_depfile_with_deps() {
        let data = b"kernel/bar.ko.xz: kernel/foo.ko.xz\n";
        let map = parse_dep_file(data);
        assert_eq!(map.get("bar.ko.xz"), Some(&vec!["foo.ko.xz".to_string()]));
    }

    #[test]
    fn test_parse_depfile_mixed() {
        let data =
            b"kernel/a.ko.xz:\nkernel/b.ko.xz: kernel/a.ko.xz\nkernel/c.ko.xz: kernel/a.ko.xz kernel/b.ko.xz\n";
        let map = parse_dep_file(data);
        let empty: Vec<String> = vec![];
        assert_eq!(map.get("a.ko.xz"), Some(&empty));
        assert_eq!(map.get("b.ko.xz"), Some(&vec!["a.ko.xz".to_string()]));
        assert_eq!(map.get("c.ko.xz"), Some(&vec!["a.ko.xz".to_string(), "b.ko.xz".to_string()]));
    }

    #[test]
    fn test_resolve_linear_deps() {
        let data =
            b"kernel/c.ko.xz:\nkernel/b.ko.xz: kernel/c.ko.xz\nkernel/a.ko.xz: kernel/b.ko.xz\n";
        let map = parse_dep_file(data);
        let order = resolve_module_order(&map, &["a.ko.xz".to_string()]).unwrap();
        assert_eq!(order, vec!["c.ko.xz", "b.ko.xz", "a.ko.xz"]);
    }

    #[test]
    fn test_resolve_shared_deps() {
        let data =
            b"kernel/c.ko.xz:\nkernel/a.ko.xz: kernel/c.ko.xz\nkernel/b.ko.xz: kernel/c.ko.xz\n";
        let map = parse_dep_file(data);
        let order =
            resolve_module_order(&map, &["a.ko.xz".to_string(), "b.ko.xz".to_string()]).unwrap();
        let c_pos = order.iter().position(|m| m == "c.ko.xz").unwrap();
        let a_pos = order.iter().position(|m| m == "a.ko.xz").unwrap();
        let b_pos = order.iter().position(|m| m == "b.ko.xz").unwrap();
        assert!(c_pos < a_pos);
        assert!(c_pos < b_pos);
    }

    #[test]
    fn test_resolve_extra_depfile_entries_ignored() {
        let map = parse_dep_file(VIRTIO_DEP);
        let order = resolve_module_order(&map, &["virtio_ring.ko.xz".to_string()]).unwrap();
        assert_eq!(order, vec!["virtio_ring.ko.xz"]);
    }

    #[test]
    fn test_resolve_virtio_net_full() {
        let map = parse_dep_file(VIRTIO_DEP);
        let order = resolve_module_order(&map, &["virtio_net.ko.xz".to_string()]).unwrap();
        let net_pos = order.iter().position(|m| m == "virtio_net.ko.xz").unwrap();
        let ring_pos = order.iter().position(|m| m == "virtio_ring.ko.xz").unwrap();
        let virtio_pos = order.iter().position(|m| m == "virtio.ko.xz").unwrap();
        let nf_pos = order.iter().position(|m| m == "net_failover.ko.xz").unwrap();
        let failover_pos = order.iter().position(|m| m == "failover.ko.xz").unwrap();
        assert!(ring_pos < net_pos);
        assert!(virtio_pos < net_pos);
        assert!(nf_pos < net_pos);
        assert!(failover_pos < net_pos);
        assert!(failover_pos < nf_pos);
        assert!(ring_pos < virtio_pos);
    }

    #[test]
    fn test_cycle_two_modules() {
        let map = parse_dep_file(CYCLE_DEP);
        let result = resolve_module_order(&map, &["virtio_net.ko.xz".to_string()]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("→"));
    }

    #[test]
    fn test_cycle_three_modules() {
        let map = parse_dep_file(CYCLE3_DEP);
        let result = resolve_module_order(&map, &["a.ko.xz".to_string()]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("a.ko.xz"));
        assert!(msg.contains("b.ko.xz"));
        assert!(msg.contains("c.ko.xz"));
    }

    #[test]
    fn test_self_loop() {
        let map = parse_dep_file(SELFLOOP_DEP);
        let result = resolve_module_order(&map, &["a.ko.xz".to_string()]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("a.ko.xz"));
    }

    #[test]
    fn test_module_not_found() {
        let map = parse_dep_file(&[]);
        let result = resolve_module_order(&map, &["nonexistent.ko.xz".to_string()]);
        assert!(result.is_err());
    }
}
