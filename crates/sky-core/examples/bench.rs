//! Release benchmark for `sky-core` YAML parsing, multi-cloud cost optimization, and spot failover.

use std::hint::black_box;
use std::time::Instant;

use sky_core::{default_cloud_catalog, Cloud, MockCloudProvider, Optimizer, SkyBroker, SkyTask};

fn bench<F: FnMut()>(name: &str, warmup: usize, iters: usize, mut f: F) {
    for _ in 0..warmup {
        f();
    }
    let mut times = Vec::with_capacity(iters);
    for _ in 0..iters {
        let t0 = Instant::now();
        f();
        times.push(t0.elapsed().as_secs_f64() * 1000.0);
    }
    times.sort_by(f64::total_cmp);
    println!(
        "{name}: median={:.4} ms (min={:.4} ms)",
        times[times.len() / 2],
        times[0]
    );
}

fn main() {
    let yaml = "name: eval-job\nresources:\n  accelerators:\n    A100: 1\n  use_spot: true\n";
    let catalog = default_cloud_catalog();
    let opt = Optimizer::new(catalog);
    let task = SkyTask::from_yaml(yaml).expect("yaml");

    bench("SkyTask::from_yaml(100x)", 3, 15, || {
        for _ in 0..100 {
            let _ = black_box(SkyTask::from_yaml(black_box(yaml)));
        }
    });

    bench("Optimizer::plan(1000x)", 3, 15, || {
        for _ in 0..1000 {
            let _ = black_box(opt.plan(black_box(&task.resources)));
        }
    });

    bench("SkyBroker::launch_with_failover(100x)", 3, 15, || {
        for _ in 0..100 {
            let provider = MockCloudProvider::with_unavailable([Cloud::RunPod]);
            let mut broker = SkyBroker::new(opt.clone(), provider);
            let _ = black_box(broker.launch(&task));
        }
    });
}
