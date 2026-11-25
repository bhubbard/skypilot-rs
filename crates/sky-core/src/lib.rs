//! Pure-Rust port of UC Berkeley Sky Computing Lab's SkyPilot compute broker.
//!
//! Provides:
//! - [`task`]: YAML-compatible `SkyTask` and `ResourceSpec` definitions.
//! - [`optimizer`]: Multi-cloud `InstanceOffer` catalog and cost-minimizing
//!   spot/on-demand failover planner.
//! - [`broker`]: `CloudProvider` trait, cluster lifecycle management, and
//!   automatic spot failover orchestration.

pub mod broker;
pub mod error;
pub mod optimizer;
pub mod task;

pub use broker::{CloudProvider, ClusterRecord, ClusterStatus, MockCloudProvider, SkyBroker};
pub use error::SkyError;
pub use optimizer::{default_cloud_catalog, Cloud, InstanceOffer, LaunchCandidate, Optimizer};
pub use task::{ResourceSpec, SkyTask};
