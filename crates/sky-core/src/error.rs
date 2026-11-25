//! Error types for `sky-core`.

use thiserror::Error;

/// Errors returned by SkyPilot task parsing, optimization, and provisioning.
#[derive(Debug, Error)]
pub enum SkyError {
    /// YAML task parsing or validation error.
    #[error("task specification error: {0}")]
    TaskYaml(#[from] serde_yaml::Error),

    /// No cloud instance offers satisfy the requested resources.
    #[error("no feasible cloud resources found: {0}")]
    NoResources(String),

    /// Provisioning failed across all failover candidates.
    #[error("provisioning failed: {0}")]
    ProvisionFailed(String),
}
