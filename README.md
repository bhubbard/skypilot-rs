# skypilot-rs

Pure-Rust port of [skypilot-org/skypilot](https://github.com/skypilot-org/skypilot) — multi-cloud GPU/CPU compute broker, SkyPilot YAML task parser, spot/on-demand cost optimizer, and automatic failover orchestrator.

Zero Python runtime required.

## Workspace Crates

| Crate | Purpose |
| :--- | :--- |
| `sky-core` | `SkyTask` YAML parser, multi-cloud `InstanceOffer` catalog, cost-minimizing `Optimizer`, `CloudProvider` trait, and `SkyBroker` spot failover |
| `sky-cli` | `sky` CLI (`optimize`, `launch-demo`) |

## Quickstart

```bash
cargo test --workspace
cargo run -p sky-cli -- launch-demo
```
