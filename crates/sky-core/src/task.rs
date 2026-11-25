//! SkyPilot YAML task and resource specification (`SkyTask`, `ResourceSpec`).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::optimizer::Cloud;
use crate::SkyError;

/// Compute resource requirements for a [`SkyTask`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceSpec {
    /// Optional preferred cloud provider.
    #[serde(default)]
    pub cloud: Option<Cloud>,
    /// Optional preferred region (e.g. `"us-west-2"`).
    #[serde(default)]
    pub region: Option<String>,
    /// Required accelerator map (e.g. `"A100": 1`, `"H100": 8`).
    #[serde(default)]
    pub accelerators: HashMap<String, usize>,
    /// Minimum vCPUs.
    #[serde(default)]
    pub cpus: Option<u32>,
    /// Minimum RAM in GiB.
    #[serde(default)]
    pub memory_gb: Option<u32>,
    /// Whether to prefer spot/preemptible instances.
    #[serde(default)]
    pub use_spot: bool,
    /// Fallback resource specifications (`any_of`).
    #[serde(default)]
    pub any_of: Vec<ResourceSpec>,
}

impl Default for ResourceSpec {
    fn default() -> Self {
        Self {
            cloud: None,
            region: None,
            accelerators: HashMap::new(),
            cpus: Some(4),
            memory_gb: Some(16),
            use_spot: true,
            any_of: Vec::new(),
        }
    }
}

/// Declarative SkyPilot job specification (compatible with SkyPilot YAML).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkyTask {
    /// Task / cluster name.
    pub name: String,
    /// Number of nodes.
    #[serde(default = "default_nodes")]
    pub num_nodes: usize,
    /// Compute resource requirements.
    #[serde(default)]
    pub resources: ResourceSpec,
    /// Local-to-remote file mount mappings.
    #[serde(default)]
    pub file_mounts: HashMap<String, String>,
    /// Environment variables.
    #[serde(default)]
    pub envs: HashMap<String, String>,
    /// Setup shell script executed once on cluster creation.
    #[serde(default)]
    pub setup: Option<String>,
    /// Main job command.
    #[serde(default)]
    pub run: Option<String>,
}

const fn default_nodes() -> usize {
    1
}

impl SkyTask {
    /// Parses a [`SkyTask`] from SkyPilot YAML.
    ///
    /// # Errors
    /// Returns [`SkyError::TaskYaml`] if the YAML is malformed.
    pub fn from_yaml(yaml: &str) -> Result<Self, SkyError> {
        Ok(serde_yaml::from_str(yaml)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_skypilot_yaml_task() {
        let yaml = r#"
name: train-qtip
num_nodes: 1
resources:
  accelerators:
    A100: 1
  use_spot: true
setup: |
  cargo build --release
run: |
  ./target/release/qtip demo
"#;
        let task = SkyTask::from_yaml(yaml).expect("valid yaml");
        assert_eq!(task.name, "train-qtip");
        assert_eq!(task.resources.accelerators.get("A100"), Some(&1));
        assert!(task.resources.use_spot);
    }
}
