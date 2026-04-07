use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use serde::Deserialize;
use snafu::ResultExt;

use crate::{
    config::{Error, error, error::ValidationError},
    graph::DiGraph,
};

/// Kernel module loading configuration.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, tag = "mode")]
pub enum ModulesConfig {
    /// Load specific modules by name.
    #[serde(rename_all = "camelCase")]
    List {
        /// Directory containing kernel modules (defaults to /lib/modules).
        #[serde(default)]
        dir: Option<String>,
        /// List of module names to load.
        names: Vec<String>,
        /// Optional path to a modules.dep file for dependency resolution.
        #[serde(default)]
        dep_file_path: Option<String>,
    },
    /// Scan directory for all .ko/.ko.xz/.ko.gz files and load each.
    #[serde(rename_all = "camelCase")]
    Scan {
        /// Directory to scan for kernel modules.
        dir: String,
        /// Path to a modules.dep file for dependency resolution.
        dep_file_path: String,
        /// Optional list of module names to filter which modules to load.
        #[serde(default)]
        names: Option<Vec<String>>,
    },
}

impl ModulesConfig {
    /// Validates module configuration.
    ///
    /// Checks that the `dep_file_path` (if provided) exists and is readable,
    /// and that all module names exist in the dependency file.
    pub fn validate(&self) -> Result<(), Error> {
        match self {
            Self::List { names, dep_file_path, .. } => {
                if let Some(dep_path) = dep_file_path {
                    let data = std::fs::read(dep_path).with_context(|_| {
                        error::ParseModuleDependencyFileSnafu { path: dep_path.clone() }
                    })?;
                    let dep_map = parse_dep_file(&data);

                    for name in names {
                        if !dep_map.contains_key(name) {
                            return Err(Error::InvalidConfig {
                                message: format!("Module not found in dep file: {name}"),
                            });
                        }
                    }
                }
                Ok(())
            }
            Self::Scan { names, dep_file_path, .. } => {
                let data = std::fs::read(dep_file_path).with_context(|_| {
                    error::ParseModuleDependencyFileSnafu { path: dep_file_path.clone() }
                })?;
                let dep_map = parse_dep_file(&data);

                if let Some(names_vec) = names {
                    for name in names_vec {
                        if !dep_map.contains_key(name) {
                            return Err(Error::InvalidConfig {
                                message: format!("Module not found in dep file: {name}"),
                            });
                        }
                    }
                }
                Ok(())
            }
        }
    }

    /// Resolves module dependencies and sorts the module names in topological
    /// order.
    ///
    /// If a `dep_file_path` is provided, reads the dependency file and sorts
    /// the module names accordingly. This modifies the internal state of
    /// the config.
    pub fn resolve_dependencies(&mut self) -> Result<(), Error> {
        match self {
            Self::List { names, dep_file_path, .. } => {
                if let Some(dep_path) = dep_file_path {
                    let path_clone = dep_path.clone();
                    let data = std::fs::read(&*dep_path).with_context(|_| {
                        error::ParseModuleDependencyFileSnafu { path: path_clone }
                    })?;
                    let dep_map = parse_dep_file(&data);
                    let sorted = resolve_module_order(&dep_map, names)
                        .with_context(|_| error::ValidateSnafu)?;
                    *names = sorted;
                }
                Ok(())
            }
            Self::Scan { names, dep_file_path, .. } => {
                let path_clone = dep_file_path.clone();
                let data = std::fs::read(&*dep_file_path)
                    .with_context(|_| error::ParseModuleDependencyFileSnafu { path: path_clone })?;
                let dep_map = parse_dep_file(&data);
                if let Some(names_vec) = names {
                    let targets = names_vec.clone();
                    let sorted = resolve_module_order(&dep_map, &targets)
                        .with_context(|_| error::ValidateSnafu)?;
                    *names_vec = sorted;
                }
                Ok(())
            }
        }
    }
}

impl From<ModulesConfig> for ocelot_bootstrap::ModulesConfig {
    fn from(mut config: ModulesConfig) -> Self {
        drop(config.resolve_dependencies());

        match config {
            ModulesConfig::List { dir, names, .. } => {
                Self::List { dir: dir.map(PathBuf::from), names }
            }
            ModulesConfig::Scan { dir: _, names, .. } => {
                let resolved_names = names.unwrap_or_default();
                Self::List { dir: None, names: resolved_names }
            }
        }
    }
}

/// Parse a `modules.dep` text file and return a mapping of module basenames to
/// their dependency basenames.
///
/// Each line has the format:
/// `kernel/path/to/module.ko.xz: kernel/path/to/dep1.ko.xz
/// kernel/path/to/dep2.ko.xz`
///
/// Returns a map from module basename (e.g., `virtio_net.ko.xz`) to a list of
/// dependency basenames.
fn parse_dep_file(data: &[u8]) -> HashMap<String, Vec<String>> {
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

/// Resolve a valid module loading order from a dependency map and a list of
/// target modules.
///
/// Performs topological sort using Kosaraju's SCC. If a cycle is detected,
/// returns a `CyclicDependency` error with the full cycle path.
///
/// Only the targets and their transitive dependencies are included in the
/// result. Extra entries in the dependency map are ignored.
fn resolve_module_order(
    dep_map: &HashMap<String, Vec<String>>,
    targets: &[String],
) -> Result<Vec<String>, ValidationError> {
    let needed = collect_transitive_deps(dep_map, targets)?;

    let mut graph = DiGraph::<String>::new();

    for name in &needed {
        let _ = graph.add_node(name, name.clone());
    }

    for name in &needed {
        if let Some(deps) = dep_map.get(name) {
            for dep in deps {
                if needed.contains(dep) {
                    graph.add_edge(name, dep);
                }
            }
        }
    }

    graph.detect_cycle().map_or_else(
        || {
            let order = graph.topological_order();
            Ok(order)
        },
        |cycle| Err(ValidationError::CyclicDependency { cycle }),
    )
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

/// Extract the basename (filename component) from a module path.
fn basename(path: &str) -> String { path.rsplit('/').next().unwrap_or(path).to_string() }

#[cfg(test)]
mod tests {
    use serde_yaml::from_str;

    use super::{ModulesConfig, parse_dep_file, resolve_module_order};

    #[test]
    fn test_deserialize_list_full() {
        let yaml = r"
mode: list
dir: /lib/modules
names:
  - foo
  - bar
depFilePath: /path/modules.dep
";
        let config: ModulesConfig = from_str(yaml).unwrap();
        assert!(
            matches!(config, ModulesConfig::List { dir: Some(d), names, dep_file_path: Some(p) } if d == "/lib/modules" && names == vec!["foo", "bar"] && p == "/path/modules.dep")
        );
    }

    #[test]
    fn test_deserialize_list_minimal() {
        let yaml = r"
mode: list
names:
  - foo
";
        let config: ModulesConfig = from_str(yaml).unwrap();
        assert!(
            matches!(config, ModulesConfig::List { dir: None, names, dep_file_path: None } if names == vec!["foo"])
        );
    }

    #[test]
    fn test_deserialize_list_dir_null() {
        let yaml = r"
mode: list
dir: null
names:
  - foo
";
        let config: ModulesConfig = from_str(yaml).unwrap();
        assert!(matches!(config, ModulesConfig::List { dir: None, names: _, dep_file_path: None }));
    }

    #[test]
    fn test_deserialize_list_names_empty() {
        let yaml = r"
mode: list
names: []
";
        let config: ModulesConfig = from_str(yaml).unwrap();
        assert!(matches!(config, ModulesConfig::List { names, .. } if names.is_empty()));
    }

    #[test]
    fn test_deserialize_scan_full() {
        let yaml = r"
mode: scan
dir: /lib/modules
depFilePath: /path/modules.dep
names:
  - foo
  - bar
";
        let config: ModulesConfig = from_str(yaml).unwrap();
        assert!(
            matches!(config, ModulesConfig::Scan { dir, dep_file_path, names } if dir == "/lib/modules" && dep_file_path == "/path/modules.dep" && names == Some(vec!["foo".to_string(), "bar".to_string()]))
        );
    }

    #[test]
    fn test_deserialize_scan_minimal() {
        let yaml = r"
mode: scan
dir: /lib/modules
depFilePath: /path/modules.dep
";
        let config: ModulesConfig = from_str(yaml).unwrap();
        assert!(
            matches!(config, ModulesConfig::Scan { dir, dep_file_path, names } if dir == "/lib/modules" && dep_file_path == "/path/modules.dep" && names.is_none())
        );
    }

    #[test]
    fn test_deserialize_scan_names_null() {
        let yaml = r"
mode: scan
dir: /lib/modules
depFilePath: /path/modules.dep
names: null
";
        let config: ModulesConfig = from_str(yaml).unwrap();
        assert!(matches!(config, ModulesConfig::Scan { names: None, .. }));
    }

    #[test]
    fn test_deserialize_list_unknown_field() {
        let yaml = r"
mode: list
names:
  - foo
unknown: true
";
        let result: Result<ModulesConfig, _> = from_str(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_list_missing_names() {
        let yaml = r"
mode: list
";
        let result: Result<ModulesConfig, _> = from_str(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_scan_missing_dir() {
        let yaml = r"
mode: scan
depFilePath: /path/modules.dep
";
        let result: Result<ModulesConfig, _> = from_str(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_scan_missing_dep_file_path() {
        let yaml = r"
mode: scan
dir: /lib/modules
";
        let result: Result<ModulesConfig, _> = from_str(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_invalid_mode() {
        let yaml = r"
mode: invalid
names:
  - foo
";
        let result: Result<ModulesConfig, _> = from_str(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_list_no_depfile() {
        let config = ModulesConfig::List {
            dir: None,
            names: vec!["foo.ko.xz".to_string()],
            dep_file_path: None,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_list_with_valid_depfile() {
        let data = br"kernel/foo.ko.xz:";
        let _map = parse_dep_file(data);

        let config = ModulesConfig::List {
            dir: None,
            names: vec!["foo.ko.xz".to_string()],
            dep_file_path: None,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_scan_requires_depfile() {
        let config = ModulesConfig::Scan {
            dir: "/lib/modules".to_string(),
            dep_file_path: "/nonexistent/path/modules.dep".to_string(),
            names: None,
        };
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_dependencies_list_no_depfile() {
        let mut config = ModulesConfig::List {
            dir: None,
            names: vec!["foo.ko.xz".to_string(), "bar.ko.xz".to_string()],
            dep_file_path: None,
        };
        let result = config.resolve_dependencies();
        assert!(result.is_ok());
        if let ModulesConfig::List { names, .. } = config {
            assert_eq!(names, vec!["foo.ko.xz".to_string(), "bar.ko.xz".to_string()]);
        }
    }

    #[test]
    fn test_resolve_dependencies_list_no_depfile_preserves_order() {
        let mut config = ModulesConfig::List {
            dir: None,
            names: vec!["a.ko.xz".to_string()],
            dep_file_path: None,
        };
        let result = config.resolve_dependencies();
        assert!(result.is_ok());
        if let ModulesConfig::List { names, .. } = config {
            assert_eq!(names, vec!["a.ko.xz".to_string()]);
        }
    }

    #[test]
    fn test_resolve_dependencies_scan_no_names() {
        let mut config = ModulesConfig::Scan {
            dir: "/lib/modules".to_string(),
            dep_file_path: "/nonexistent/modules.dep".to_string(),
            names: None,
        };
        let result = config.resolve_dependencies();
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_empty_depfile() {
        let map = parse_dep_file(&[]);
        assert!(map.is_empty());
    }

    #[test]
    fn test_parse_depfile_no_deps() {
        let data = b"kernel/foo.ko.xz:";
        let map = parse_dep_file(data);
        let empty: Vec<String> = vec![];
        assert_eq!(map.get("foo.ko.xz"), Some(&empty));
    }

    #[test]
    fn test_parse_depfile_with_deps() {
        let data = b"kernel/bar.ko.xz: kernel/foo.ko.xz";
        let map = parse_dep_file(data);
        assert_eq!(map.get("bar.ko.xz"), Some(&vec!["foo.ko.xz".to_string()]));
    }

    #[test]
    fn test_parse_depfile_mixed() {
        let data = br"
kernel/a.ko.xz:
kernel/b.ko.xz: kernel/a.ko.xz
kernel/c.ko.xz: kernel/a.ko.xz kernel/b.ko.xz
";
        let map = parse_dep_file(data);
        let empty: Vec<String> = vec![];
        assert_eq!(map.get("a.ko.xz"), Some(&empty));
        assert_eq!(map.get("b.ko.xz"), Some(&vec!["a.ko.xz".to_string()]));
        assert_eq!(map.get("c.ko.xz"), Some(&vec!["a.ko.xz".to_string(), "b.ko.xz".to_string()]));
    }

    #[test]
    fn test_resolve_linear_deps() {
        let data = br"
kernel/c.ko.xz:
kernel/b.ko.xz: kernel/c.ko.xz
kernel/a.ko.xz: kernel/b.ko.xz
";
        let map = parse_dep_file(data);
        let order = resolve_module_order(&map, &["a.ko.xz".to_string()]).unwrap();
        assert_eq!(order, vec!["c.ko.xz", "b.ko.xz", "a.ko.xz"]);
    }

    #[test]
    fn test_resolve_shared_deps() {
        let data = br"
kernel/c.ko.xz:
kernel/a.ko.xz: kernel/c.ko.xz
kernel/b.ko.xz: kernel/c.ko.xz
";
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
        let data = br"
kernel/drivers/virtio/virtio_ring.ko.xz:
kernel/drivers/virtio/virtio.ko.xz: kernel/drivers/virtio/virtio_ring.ko.xz
kernel/drivers/net/net_failover.ko.xz: kernel/net/core/failover.ko.xz
kernel/net/core/failover.ko.xz:
kernel/drivers/virtio/virtio_mmio.ko.xz: kernel/drivers/virtio/virtio.ko.xz kernel/drivers/virtio/virtio_ring.ko.xz
kernel/drivers/virtio/virtio_pci.ko.xz: kernel/drivers/virtio/virtio_pci_legacy_dev.ko.xz kernel/drivers/virtio/virtio_pci_modern_dev.ko.xz kernel/drivers/virtio/virtio.ko.xz kernel/drivers/virtio/virtio_ring.ko.xz
kernel/drivers/virtio/virtio_pci_legacy_dev.ko.xz:
kernel/drivers/virtio/virtio_pci_modern_dev.ko.xz:
kernel/drivers/net/virtio_net.ko.xz: kernel/drivers/net/net_failover.ko.xz kernel/net/core/failover.ko.xz kernel/drivers/virtio/virtio.ko.xz kernel/drivers/virtio/virtio_ring.ko.xz
kernel/drivers/block/virtio_blk.ko.xz: kernel/drivers/virtio/virtio.ko.xz kernel/drivers/virtio/virtio_ring.ko.xz
kernel/fs/fuse/fuse.ko.xz:
kernel/fs/fuse/virtiofs.ko.xz: kernel/drivers/virtio/virtio.ko.xz kernel/drivers/virtio/virtio_ring.ko.xz kernel/fs/fuse/fuse.ko.xz
";
        let map = parse_dep_file(data);
        let order = resolve_module_order(&map, &["virtio_ring.ko.xz".to_string()]).unwrap();
        assert_eq!(order, vec!["virtio_ring.ko.xz"]);
    }

    #[test]
    fn test_resolve_virtio_net_full() {
        let data = br"
kernel/drivers/virtio/virtio_ring.ko.xz:
kernel/drivers/virtio/virtio.ko.xz: kernel/drivers/virtio/virtio_ring.ko.xz
kernel/drivers/net/net_failover.ko.xz: kernel/net/core/failover.ko.xz
kernel/net/core/failover.ko.xz:
kernel/drivers/virtio/virtio_mmio.ko.xz: kernel/drivers/virtio/virtio.ko.xz kernel/drivers/virtio/virtio_ring.ko.xz
kernel/drivers/virtio/virtio_pci.ko.xz: kernel/drivers/virtio/virtio_pci_legacy_dev.ko.xz kernel/drivers/virtio/virtio_pci_modern_dev.ko.xz kernel/drivers/virtio/virtio.ko.xz kernel/drivers/virtio/virtio_ring.ko.xz
kernel/drivers/virtio/virtio_pci_legacy_dev.ko.xz:
kernel/drivers/virtio/virtio_pci_modern_dev.ko.xz:
kernel/drivers/net/virtio_net.ko.xz: kernel/drivers/net/net_failover.ko.xz kernel/net/core/failover.ko.xz kernel/drivers/virtio/virtio.ko.xz kernel/drivers/virtio/virtio_ring.ko.xz
kernel/drivers/block/virtio_blk.ko.xz: kernel/drivers/virtio/virtio.ko.xz kernel/drivers/virtio/virtio_ring.ko.xz
kernel/fs/fuse/fuse.ko.xz:
kernel/fs/fuse/virtiofs.ko.xz: kernel/drivers/virtio/virtio.ko.xz kernel/drivers/virtio/virtio_ring.ko.xz kernel/fs/fuse/fuse.ko.xz
";
        let map = parse_dep_file(data);
        let order = resolve_module_order(&map, &["virtio_net.ko.xz".to_string()]).unwrap();
        let net_pos = order.iter().position(|m| m == "virtio_net.ko.xz").unwrap();
        let ring_pos = order.iter().position(|m| m == "virtio_ring.ko.xz").unwrap();
        let virtio_pos = order.iter().position(|m| m == "virtio.ko.xz").unwrap();
        let nf_pos = order.iter().position(|m| m == "net_failover.ko.xz").unwrap();
        let failover_pos = order.iter().position(|m| m == "failover.ko.xz").unwrap();
        assert!(ring_pos < net_pos);
        assert!(virtio_pos < net_pos);
        assert!(nf_pos < net_pos);
        assert!(failover_pos < nf_pos);
        assert!(failover_pos < nf_pos);
        assert!(ring_pos < virtio_pos);
    }

    #[test]
    fn test_cycle_two_modules() {
        let data = br"
kernel/drivers/net/virtio_net.ko.xz: kernel/drivers/virtio/virtio.ko.xz
kernel/drivers/virtio/virtio.ko.xz: kernel/drivers/net/virtio_net.ko.xz
";
        let map = parse_dep_file(data);
        let result = resolve_module_order(&map, &["virtio_net.ko.xz".to_string()]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("→"));
    }

    #[test]
    fn test_cycle_three_modules() {
        let data = br"
kernel/a.ko.xz: kernel/b.ko.xz
kernel/b.ko.xz: kernel/c.ko.xz
kernel/c.ko.xz: kernel/a.ko.xz
";
        let map = parse_dep_file(data);
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
        let data = br"kernel/a.ko.xz: kernel/a.ko.xz";
        let map = parse_dep_file(data);
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
