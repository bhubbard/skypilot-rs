//! Multi-cloud compute broker and automatic spot failover orchestrator.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::optimizer::{Cloud, LaunchCandidate, Optimizer};
use crate::task::SkyTask;
use crate::SkyError;

/// Lifecycle status of a provisioned cluster.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClusterStatus {
    /// Initializing / provisioning.
    Init,
    /// Active and ready.
    Up,
    /// Stopped / hibernated.
    Stopped,
    /// Preempted by cloud spot market.
    Preempted,
}

/// Metadata record for an active or historical cluster.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClusterRecord {
    /// Cluster name.
    pub name: String,
    /// Active cloud + instance selection.
    pub candidate: LaunchCandidate,
    /// Current lifecycle state.
    pub status: ClusterStatus,
    /// Number of failover hops performed.
    pub failover_attempts: usize,
}

/// Backend cloud provisioning interface.
pub trait CloudProvider {
    /// Attempts to provision `candidate` for `cluster_name`.
    ///
    /// # Errors
    /// Returns [`SkyError::ProvisionFailed`] if capacity or quota is unavailable.
    fn provision(
        &mut self,
        cluster_name: &str,
        candidate: &LaunchCandidate,
    ) -> Result<(), SkyError>;

    /// Terminates a running cluster.
    ///
    /// # Errors
    /// Returns [`SkyError::ProvisionFailed`] if teardown fails.
    fn terminate(&mut self, cluster_name: &str) -> Result<(), SkyError>;
}

/// Deterministic mock [`CloudProvider`] supporting simulated spot capacity
/// failures to verify multi-cloud failover.
#[derive(Debug, Default)]
pub struct MockCloudProvider {
    unavailable_clouds: HashSet<Cloud>,
    active: HashMap<String, LaunchCandidate>,
}

impl MockCloudProvider {
    /// Creates a [`MockCloudProvider`] where `unavailable_clouds` reject provisioning.
    #[must_use]
    pub fn with_unavailable(unavailable: impl IntoIterator<Item = Cloud>) -> Self {
        Self {
            unavailable_clouds: unavailable.into_iter().collect(),
            active: HashMap::new(),
        }
    }
}

impl CloudProvider for MockCloudProvider {
    fn provision(
        &mut self,
        cluster_name: &str,
        candidate: &LaunchCandidate,
    ) -> Result<(), SkyError> {
        if self.unavailable_clouds.contains(&candidate.offer.cloud) {
            return Err(SkyError::ProvisionFailed(format!(
                "spot capacity exhausted on {:?}",
                candidate.offer.cloud
            )));
        }
        self.active
            .insert(cluster_name.to_string(), candidate.clone());
        Ok(())
    }

    fn terminate(&mut self, cluster_name: &str) -> Result<(), SkyError> {
        self.active.remove(cluster_name);
        Ok(())
    }
}

/// SkyPilot multi-cloud broker managing optimization, failover launch, and teardown.
pub struct SkyBroker<P> {
    optimizer: Optimizer,
    provider: P,
    clusters: HashMap<String, ClusterRecord>,
}

impl<P: CloudProvider> SkyBroker<P> {
    /// Creates a new [`SkyBroker`].
    #[must_use]
    pub fn new(optimizer: Optimizer, provider: P) -> Self {
        Self {
            optimizer,
            provider,
            clusters: HashMap::new(),
        }
    }

    /// Plans and launches `task`, automatically failing over across ranked
    /// cloud/region/spot candidates if earlier providers reject capacity.
    ///
    /// # Errors
    /// Returns [`SkyError`] if no offer matches or all failover candidates fail.
    pub fn launch(&mut self, task: &SkyTask) -> Result<ClusterRecord, SkyError> {
        let plan = self.optimizer.plan(&task.resources)?;
        let mut last_err = None;
        for (idx, candidate) in plan.into_iter().enumerate() {
            match self.provider.provision(&task.name, &candidate) {
                Ok(()) => {
                    let record = ClusterRecord {
                        name: task.name.clone(),
                        candidate,
                        status: ClusterStatus::Up,
                        failover_attempts: idx,
                    };
                    self.clusters.insert(task.name.clone(), record.clone());
                    return Ok(record);
                }
                Err(err) => {
                    last_err = Some(err);
                }
            }
        }
        Err(last_err.unwrap_or_else(|| SkyError::ProvisionFailed("empty failover plan".into())))
    }

    /// Terminates cluster `name` and removes it from active state.
    ///
    /// # Errors
    /// Returns [`SkyError`] if provider termination fails.
    pub fn down(&mut self, name: &str) -> Result<(), SkyError> {
        self.provider.terminate(name)?;
        self.clusters.remove(name);
        Ok(())
    }

    /// Lists all active clusters.
    #[must_use]
    pub fn status(&self) -> Vec<&ClusterRecord> {
        self.clusters.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimizer::default_cloud_catalog;

    #[test]
    fn broker_fails_over_when_cheapest_cloud_is_exhausted() {
        let task = SkyTask::from_yaml(
            r#"
name: eval-cluster
resources:
  accelerators:
    A100: 1
  use_spot: true
"#,
        )
        .expect("valid yaml");

        let provider = MockCloudProvider::with_unavailable([Cloud::RunPod]);
        let mut broker = SkyBroker::new(Optimizer::new(default_cloud_catalog()), provider);
        let rec = broker.launch(&task).expect("failover to GCP succeeds");
        assert_eq!(rec.candidate.offer.cloud, Cloud::Gcp);
        assert_eq!(rec.failover_attempts, 1);
        assert_eq!(rec.status, ClusterStatus::Up);

        broker.down("eval-cluster").expect("down succeeds");
        assert!(broker.status().is_empty());
    }
}
