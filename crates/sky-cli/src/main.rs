//! CLI for SkyPilot multi-cloud compute optimization and spot failover.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use sky_core::{default_cloud_catalog, Cloud, MockCloudProvider, Optimizer, SkyBroker, SkyTask};

#[derive(Parser)]
#[command(
    name = "sky",
    about = "Pure-Rust SkyPilot multi-cloud compute broker CLI"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Plan the cheapest multi-cloud failover schedule for a SkyPilot YAML task.
    Optimize {
        #[arg(long)]
        file: PathBuf,
    },
    /// Simulate launching a SkyPilot YAML task with spot failover.
    LaunchDemo {
        #[arg(long)]
        file: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Optimize { file } => {
            let yaml =
                fs::read_to_string(&file).with_context(|| format!("reading {}", file.display()))?;
            let task = SkyTask::from_yaml(&yaml)?;
            let opt = Optimizer::new(default_cloud_catalog());
            let plan = opt.plan(&task.resources)?;
            println!("{}", serde_json::to_string_pretty(&plan)?);
        }
        Commands::LaunchDemo { file } => {
            let task = if let Some(path) = file {
                let yaml = fs::read_to_string(&path)?;
                SkyTask::from_yaml(&yaml)?
            } else {
                SkyTask::from_yaml(
                    "name: demo-job\nresources:\n  accelerators:\n    A100: 1\n  use_spot: true\n",
                )?
            };
            let provider = MockCloudProvider::with_unavailable([Cloud::RunPod]);
            let mut broker = SkyBroker::new(Optimizer::new(default_cloud_catalog()), provider);
            let record = broker.launch(&task)?;
            println!("{}", serde_json::to_string_pretty(&record)?);
        }
    }
    Ok(())
}
