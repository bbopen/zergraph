//! Dependency-free, release-mode performance smoke harness.
//!
//! `cargo bench --bench perf -- --measure` prints TSV only. Tune fixture sizes with
//! `ZERGRAPH_BENCH_SMALL_N`, `ZERGRAPH_BENCH_MEDIUM_N`, and
//! `ZERGRAPH_BENCH_DEGREE`; tune repeated samples with
//! `ZERGRAPH_BENCH_SAMPLES`, or select comma-separated metrics with
//! `ZERGRAPH_BENCH_ONLY=node_lookup,snapshot_encode`. Defaults are deliberately
//! conservative: seven samples over 64/1,024-node sparse graphs normally finish
//! well below 30 s. `snapshot`, `fork`, `snapshot_encode`, and `snapshot_decode`
//! include owned-result destruction; `restore` and `merge_changed_property`
//! exclude fixture preparation and result destruction so they isolate their named
//! operation.

use std::{
    env,
    hint::black_box,
    time::{Duration, Instant},
};
use zergraph::{EdgeKey, Graph, Snapshot};

const DEFAULT_SMALL_N: usize = 64;
const DEFAULT_MEDIUM_N: usize = 1_024;
const DEFAULT_DEGREE: usize = 4;
const DEFAULT_SAMPLES: usize = 7;

struct Fixture {
    graph: Graph,
    ids: Vec<String>,
    snapshot: Snapshot,
    bytes: Vec<u8>,
}

fn positive_env(name: &str, default: usize) -> usize {
    match env::var(name) {
        Ok(value) => value
            .parse::<usize>()
            .ok()
            .filter(|value| *value > 0)
            .unwrap_or_else(|| panic!("{name} must be a positive integer, got {value:?}")),
        Err(_) => default,
    }
}

fn graph_fixture(node_count: usize, requested_degree: usize) -> (Graph, Vec<String>) {
    let node_count = node_count.max(2);
    let degree = requested_degree.clamp(1, node_count - 1);
    let ids: Vec<_> = (0..node_count)
        .map(|index| format!("node:{index:06}"))
        .collect();
    let mut graph = Graph::new();

    for (index, id) in ids.iter().enumerate() {
        graph.add_node(id).unwrap();
        graph.set_node_property(id, "fixture", "perf").unwrap();
        graph.set_node_property(id, "index", index as u64).unwrap();
    }
    for source in 0..node_count {
        for offset in 1..=degree {
            let target = (source + offset) % node_count;
            let edge = EdgeKey::new(&ids[source], format!("relation:{offset}"), &ids[target]);
            graph.add_edge(edge.clone()).unwrap();
            graph
                .set_edge_property(&edge, "weight", offset as u64)
                .unwrap();
        }
    }

    (graph, ids)
}

fn fixture(node_count: usize, requested_degree: usize) -> Fixture {
    let (graph, ids) = graph_fixture(node_count, requested_degree);
    let snapshot = graph.snapshot();
    let bytes = snapshot.to_bytes().unwrap();
    Fixture {
        graph,
        ids,
        snapshot,
        bytes,
    }
}

fn measure<T>(work: impl FnOnce() -> T) -> Duration {
    let started = Instant::now();
    black_box(work());
    started.elapsed()
}

fn median(mut samples: Vec<Duration>) -> Duration {
    samples.sort_unstable();
    samples[samples.len() / 2]
}

fn report(
    fixture: &str,
    metric: &str,
    iterations: usize,
    snapshot_bytes: usize,
    samples: Vec<Duration>,
) {
    for (sample, duration) in samples.iter().enumerate() {
        let total_ns = duration.as_nanos();
        println!(
            "sample\t{fixture}\t{metric}\t{sample}\t{iterations}\t{total_ns}\t{:.3}\t{snapshot_bytes}",
            total_ns as f64 / iterations as f64
        );
    }
    let median = median(samples);
    let total_ns = median.as_nanos();
    println!(
        "median\t{fixture}\t{metric}\t-\t{iterations}\t{total_ns}\t{:.3}\t{snapshot_bytes}",
        total_ns as f64 / iterations as f64
    );
}

fn repeated(samples: usize, mut work: impl FnMut() -> Duration) -> Vec<Duration> {
    (0..samples).map(|_| work()).collect()
}

fn iterations_for(nodes: usize, metric: &str) -> usize {
    // Large-fixture checks are intended as practical smoke measurements, not a
    // stress test. The usual small/medium profile retains enough repetitions for
    // a stable median while a caller can opt into a larger fixture safely.
    if nodes > DEFAULT_MEDIUM_N {
        return match metric {
            "node_lookup" => 10_000,
            "outgoing" | "incoming" => 100,
            _ => 1,
        };
    }
    let medium = nodes > 256;
    match (medium, metric) {
        (_, "node_lookup") => {
            if medium {
                20_000
            } else {
                100_000
            }
        }
        (_, "outgoing" | "incoming") => {
            if medium {
                800
            } else {
                5_000
            }
        }
        (_, "snapshot" | "fork") => {
            if medium {
                200
            } else {
                2_000
            }
        }
        (_, "restore") => {
            if medium {
                100
            } else {
                1_000
            }
        }
        (_, "merge_unchanged") => {
            if medium {
                100
            } else {
                1_000
            }
        }
        (_, "merge_changed_property") => {
            if medium {
                32
            } else {
                200
            }
        }
        (_, "snapshot_encode" | "snapshot_decode") => {
            if medium {
                50
            } else {
                500
            }
        }
        _ => unreachable!("every benchmark metric has a conservative loop count"),
    }
}

fn selected(only: &[String], metric: &str) -> bool {
    only.is_empty() || only.iter().any(|candidate| candidate == metric)
}

fn run_fixture(name: &str, node_count: usize, degree: usize, sample_count: usize, only: &[String]) {
    let fixture = fixture(node_count, degree);
    let target = &fixture.ids[fixture.ids.len() / 2];
    let snapshot_bytes = fixture.bytes.len();
    println!("value\t{name}\tsnapshot_bytes\t-\t0\t0\t0.000\t{snapshot_bytes}");

    if selected(only, "node_lookup") {
        let iterations = iterations_for(fixture.ids.len(), "node_lookup");
        report(
            name,
            "node_lookup",
            iterations,
            snapshot_bytes,
            repeated(sample_count, || {
                measure(|| {
                    let mut checksum = 0usize;
                    for _ in 0..iterations {
                        checksum ^= fixture.graph.node(black_box(target)).unwrap().id().len();
                    }
                    checksum
                })
            }),
        );
    }

    if selected(only, "outgoing") {
        let iterations = iterations_for(fixture.ids.len(), "outgoing");
        report(
            name,
            "outgoing",
            iterations,
            snapshot_bytes,
            repeated(sample_count, || {
                measure(|| {
                    let mut checksum = 0usize;
                    for _ in 0..iterations {
                        checksum += fixture.graph.outgoing(black_box(target)).count();
                    }
                    checksum
                })
            }),
        );
    }

    if selected(only, "incoming") {
        let iterations = iterations_for(fixture.ids.len(), "incoming");
        report(
            name,
            "incoming",
            iterations,
            snapshot_bytes,
            repeated(sample_count, || {
                measure(|| {
                    let mut checksum = 0usize;
                    for _ in 0..iterations {
                        checksum += fixture.graph.incoming(black_box(target)).count();
                    }
                    checksum
                })
            }),
        );
    }

    if selected(only, "snapshot") {
        let iterations = iterations_for(fixture.ids.len(), "snapshot");
        report(
            name,
            "snapshot",
            iterations,
            snapshot_bytes,
            repeated(sample_count, || {
                measure(|| {
                    for _ in 0..iterations {
                        // This measures the owning public API call and its result drop.
                        black_box(fixture.graph.snapshot());
                    }
                })
            }),
        );
    }

    if selected(only, "restore") {
        let iterations = iterations_for(fixture.ids.len(), "restore");
        report(
            name,
            "restore",
            iterations,
            snapshot_bytes,
            repeated(sample_count, || {
                let mut total = Duration::ZERO;
                for _ in 0..iterations {
                    let snapshot = fixture.snapshot.clone();
                    let started = Instant::now();
                    let restored = Graph::from_snapshot(snapshot);
                    black_box(&restored);
                    total += started.elapsed();
                    drop(restored);
                }
                total
            }),
        );
    }

    if selected(only, "fork") {
        let iterations = iterations_for(fixture.ids.len(), "fork");
        report(
            name,
            "fork",
            iterations,
            snapshot_bytes,
            repeated(sample_count, || {
                measure(|| {
                    for _ in 0..iterations {
                        // Like `snapshot`, this includes the owning result drop.
                        black_box(fixture.graph.fork());
                    }
                })
            }),
        );
    }

    if selected(only, "merge_unchanged") {
        let iterations = iterations_for(fixture.ids.len(), "merge_unchanged");
        report(
            name,
            "merge_unchanged",
            iterations,
            snapshot_bytes,
            repeated(sample_count, || {
                let mut receiver = Graph::from_snapshot(fixture.snapshot.clone());
                let peer = fixture.snapshot.clone();
                measure(|| {
                    let mut changed = 0usize;
                    for _ in 0..iterations {
                        changed += receiver.merge(black_box(&peer)).unwrap() as usize;
                    }
                    changed
                })
            }),
        );
    }

    if selected(only, "merge_changed_property") {
        let iterations = iterations_for(fixture.ids.len(), "merge_changed_property");
        report(
            name,
            "merge_changed_property",
            iterations,
            snapshot_bytes,
            repeated(sample_count, || {
                // Prepare the changed source outside timing. Each receiver is
                // prepared and destroyed one at a time so large fixtures do not
                // create a batch-sized peak allocation.
                let mut writer = Graph::from_snapshot(fixture.snapshot.clone());
                writer
                    .set_node_property(target, "merge_probe", true)
                    .unwrap();
                let changed_peer = writer.snapshot();
                let mut total = Duration::ZERO;
                for _ in 0..iterations {
                    let mut receiver = Graph::from_snapshot(fixture.snapshot.clone());
                    let started = Instant::now();
                    let changed = receiver.merge(black_box(&changed_peer)).unwrap();
                    black_box(&receiver);
                    total += started.elapsed();
                    drop(receiver);
                    black_box(changed);
                }
                total
            }),
        );
    }

    if selected(only, "snapshot_encode") {
        let iterations = iterations_for(fixture.ids.len(), "snapshot_encode");
        report(
            name,
            "snapshot_encode",
            iterations,
            snapshot_bytes,
            repeated(sample_count, || {
                measure(|| {
                    let mut checksum = 0usize;
                    for _ in 0..iterations {
                        let bytes = fixture.snapshot.to_bytes().unwrap();
                        checksum ^= bytes.len();
                        black_box(bytes);
                    }
                    checksum
                })
            }),
        );
    }

    if selected(only, "snapshot_decode") {
        let iterations = iterations_for(fixture.ids.len(), "snapshot_decode");
        report(
            name,
            "snapshot_decode",
            iterations,
            snapshot_bytes,
            repeated(sample_count, || {
                measure(|| {
                    for _ in 0..iterations {
                        // This includes destruction of the decoded owned snapshot.
                        black_box(Snapshot::from_bytes(black_box(&fixture.bytes)).unwrap());
                    }
                })
            }),
        );
    }
}

fn main() {
    // A separate-process retained-memory probe: one graph and its ID input list,
    // no snapshot clones or timing batches. Use an OS RSS measurement tool.
    if env::args().any(|argument| argument == "--memory") {
        let nodes = positive_env("ZERGRAPH_BENCH_MEDIUM_N", DEFAULT_MEDIUM_N);
        let degree = positive_env("ZERGRAPH_BENCH_DEGREE", DEFAULT_DEGREE);
        let (graph, ids) = graph_fixture(nodes, degree);
        println!(
            "nodes={} edges={} input_ids={}",
            graph.nodes().count(),
            graph.edges().count(),
            ids.len()
        );
        black_box((&graph, &ids));
        return;
    }
    // Cargo may execute harness-free benches during tests without a flag. Require
    // explicit opt-in so normal test/build cycles never start a timing campaign.
    if !env::args().any(|argument| argument == "--measure") {
        let smoke = fixture(2, 1);
        assert_eq!(smoke.graph.nodes().count(), 2);
        assert_eq!(smoke.graph.edges().count(), 2);
        assert!(!smoke.bytes.is_empty());
        println!("kind\tfixture\tmetric\tsample\titerations\ttotal_ns\tns_per_op\tsnapshot_bytes");
        println!(
            "value\tsmoke\tsnapshot_bytes\t-\t0\t0\t0.000\t{}",
            smoke.bytes.len()
        );
        return;
    }

    let small = positive_env("ZERGRAPH_BENCH_SMALL_N", DEFAULT_SMALL_N);
    let medium = positive_env("ZERGRAPH_BENCH_MEDIUM_N", DEFAULT_MEDIUM_N);
    let degree = positive_env("ZERGRAPH_BENCH_DEGREE", DEFAULT_DEGREE);
    let samples = positive_env("ZERGRAPH_BENCH_SAMPLES", DEFAULT_SAMPLES);
    let only: Vec<_> = env::var("ZERGRAPH_BENCH_ONLY")
        .ok()
        .into_iter()
        .flat_map(|metrics| {
            metrics
                .split(',')
                .map(str::trim)
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .filter(|metric| !metric.is_empty())
        .collect();

    println!("kind\tfixture\tmetric\tsample\titerations\ttotal_ns\tns_per_op\tsnapshot_bytes");
    run_fixture("small", small, degree, samples, &only);
    run_fixture("medium", medium, degree, samples, &only);
}
