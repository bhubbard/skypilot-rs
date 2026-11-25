//! Multi-cloud instance catalog and cost-minimizing failover optimizer.

use serde::{Deserialize, Serialize};

use crate::task::ResourceSpec;
use crate::SkyError;

/// Supported cloud and local compute providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Cloud {
    /// Amazon Web Services.
    Aws,
    /// Google Cloud Platform.
    Gcp,
    /// Microsoft Azure.
    Azure,
    /// Lambda Cloud.
    Lambda,
    /// RunPod GPU Cloud.
    RunPod,
    /// Kubernetes cluster.
    Kubernetes,
    /// Local workstation / ZimaBoard / Apple Silicon node.
    Local,
}

/// A concrete cloud instance SKU with spot and on-demand hourly pricing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstanceOffer {
    /// Cloud provider.
    pub cloud: Cloud,
    /// Region identifier.
    pub region: String,
    /// Instance type SKU.
    pub instance_type: String,
    /// Accelerator model (e.g. `"A100"`, `"H100"`, `"L4"`), if GPU-equipped.
    pub accelerator: Option<String>,
    /// Number of accelerators on this instance.
    pub accelerator_count: usize,
    /// Virtual CPUs.
    pub vcpus: u32,
    /// RAM in GiB.
    pub memory_gb: u32,
    /// On-demand hourly price (USD).
    pub on_demand_per_hr: f64,
    /// Spot/preemptible hourly price (USD), if offered.
    pub spot_per_hr: Option<f64>,
}

/// Ranked launch candidate produced by [`Optimizer::plan`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LaunchCandidate {
    /// Selected cloud offer.
    pub offer: InstanceOffer,
    /// Whether this candidate launches as spot.
    pub is_spot: bool,
    /// Effective hourly cost in USD.
    pub hourly_cost_usd: f64,
}

/// Multi-cloud compute optimizer ranking feasible offers by cost.
#[derive(Debug, Clone)]
pub struct Optimizer {
    catalog: Vec<InstanceOffer>,
}

impl Optimizer {
    /// Creates an [`Optimizer`] backed by `catalog`.
    #[must_use]
    pub fn new(catalog: Vec<InstanceOffer>) -> Self {
        Self { catalog }
    }

    /// Computes a cost-ordered failover plan of [`LaunchCandidate`]s matching `spec`.
    ///
    /// # Errors
    /// Returns [`SkyError::NoResources`] if no catalog offer satisfies `spec`.
    pub fn plan(&self, spec: &ResourceSpec) -> Result<Vec<LaunchCandidate>, SkyError> {
        let specs: Vec<&ResourceSpec> = if spec.any_of.is_empty() {
            vec![spec]
        } else {
            spec.any_of.iter().collect()
        };

        let mut candidates = Vec::new();
        for offer in &self.catalog {
            for s in &specs {
                if !matches_spec(offer, s) {
                    continue;
                }
                if s.use_spot {
                    if let Some(spot) = offer.spot_per_hr {
                        candidates.push(LaunchCandidate {
                            offer: offer.clone(),
                            is_spot: true,
                            hourly_cost_usd: spot,
                        });
                        continue;
                    }
                }
                candidates.push(LaunchCandidate {
                    offer: offer.clone(),
                    is_spot: false,
                    hourly_cost_usd: offer.on_demand_per_hr,
                });
            }
        }

        if candidates.is_empty() {
            return Err(SkyError::NoResources(
                "no catalog offers satisfy the requested accelerators/CPU/memory".into(),
            ));
        }

        candidates.sort_by(|a, b| {
            a.hourly_cost_usd
                .partial_cmp(&b.hourly_cost_usd)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(candidates)
    }
}

fn matches_spec(offer: &InstanceOffer, spec: &ResourceSpec) -> bool {
    if let Some(cloud) = spec.cloud {
        if offer.cloud != cloud {
            return false;
        }
    }
    if let Some(ref region) = spec.region {
        if &offer.region != region {
            return false;
        }
    }
    if let Some(cpus) = spec.cpus {
        if offer.vcpus < cpus {
            return false;
        }
    }
    if let Some(mem) = spec.memory_gb {
        if offer.memory_gb < mem {
            return false;
        }
    }
    for (acc, &count) in &spec.accelerators {
        if offer.accelerator.as_deref() != Some(acc.as_str()) || offer.accelerator_count < count {
            return false;
        }
    }
    true
}

/// Returns a built-in multi-cloud GPU/CPU catalog for optimization and testing.
#[must_use]
pub fn default_cloud_catalog() -> Vec<InstanceOffer> {
    vec![
        InstanceOffer {
            cloud: Cloud::Aws,
            region: "us-east-1".into(),
            instance_type: "p4d.24xlarge".into(),
            accelerator: Some("A100".into()),
            accelerator_count: 8,
            vcpus: 96,
            memory_gb: 1152,
            on_demand_per_hr: 32.77,
            spot_per_hr: Some(11.40),
        },
        InstanceOffer {
            cloud: Cloud::Gcp,
            region: "us-central1".into(),
            instance_type: "a2-highgpu-1g".into(),
            accelerator: Some("A100".into()),
            accelerator_count: 1,
            vcpus: 12,
            memory_gb: 85,
            on_demand_per_hr: 3.67,
            spot_per_hr: Some(1.10),
        },
        InstanceOffer {
            cloud: Cloud::RunPod,
            region: "us-tx-3".into(),
            instance_type: "A100-80GB".into(),
            accelerator: Some("A100".into()),
            accelerator_count: 1,
            vcpus: 16,
            memory_gb: 80,
            on_demand_per_hr: 1.89,
            spot_per_hr: Some(0.99),
        },
        InstanceOffer {
            cloud: Cloud::Aws,
            region: "us-west-2".into(),
            instance_type: "g5.xlarge".into(),
            accelerator: Some("A10G".into()),
            accelerator_count: 1,
            vcpus: 4,
            memory_gb: 16,
            on_demand_per_hr: 1.006,
            spot_per_hr: Some(0.35),
        },
        InstanceOffer {
            cloud: Cloud::Local,
            region: "home-unifi".into(),
            instance_type: "zimaboard-cluster".into(),
            accelerator: None,
            accelerator_count: 0,
            vcpus: 8,
            memory_gb: 16,
            on_demand_per_hr: 0.0,
            spot_per_hr: None,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn optimizer_picks_cheapest_spot_across_clouds() {
        let opt = Optimizer::new(default_cloud_catalog());
        let mut acc = HashMap::new();
        acc.insert("A100".into(), 1);
        let spec = ResourceSpec {
            accelerators: acc,
            use_spot: true,
            ..ResourceSpec::default()
        };
        let plan = opt.plan(&spec).expect("plan succeeds");
        assert_eq!(plan[0].offer.cloud, Cloud::RunPod);
        assert!(plan[0].is_spot);
        assert!((plan[0].hourly_cost_usd - 0.99).abs() < 1e-6);
    }
}
