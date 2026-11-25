# Benchmarks & Coverage

Measured on **Apple M3 Pro** (`arm64`, 11 cores, 18 GiB RAM, macOS) with `rustc 1.100.0` (`--release`) vs. `Python 3.15.2` (same host) and upstream `skypilot-org/skypilot`.

## 1. Code Coverage (`cargo llvm-cov --workspace --summary-only`)

| File | Region Coverage | Function Coverage | Line Coverage |
| :--- | ---: | ---: | ---: |
| `sky-core/src/task.rs` | 96.30% | 100.00% | 100.00% |
| `sky-core/src/broker.rs` | 93.75% | 88.89% | 97.40% |
| `sky-core/src/optimizer.rs` | 87.70% | 100.00% | 86.43% |
| **Workspace Total** *(incl. CLI)* | **78.78%** | **85.71%** | **84.62%** |

## 2. Footprint, Startup & Microbenchmarks

| Metric / Operation | `skypilot-rs` (Rust) | Upstream `skypilot-org/skypilot` (Python) | Comparison |
| :--- | ---: | ---: | :--- |
| **Standalone Binary Size** | **1.28 MiB** | Heavy Python + cloud SDKs (`boto3`, `google-cloud`, `ray`) | Single lightweight CLI |
| **Cold Startup (5-run median)** | **5.01 ms** | **23.61 ms** (`python3 -c ""` bare stdlib) | **4.71× faster** cold start |
| `SkyTask::from_yaml(100x)` | **0.4072 ms** | Python `pyyaml` + schema validation | ~4.1 µs per YAML |
| `Optimizer::plan(1000x)` | **0.2675 ms** | Python optimizer catalog scan | ~267 ns per plan |
| `SkyBroker::launch_with_failover(100x)` | **0.1099 ms** | Python provisioning state machine | ~1.1 µs per failover |

Reproduce locally:
```bash
cargo run --release -p sky-core --example bench
cargo llvm-cov --workspace --summary-only
```
