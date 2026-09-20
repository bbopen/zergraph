//! Sync timings opt in with `--measure`; ordinary test runs only check a tiny fixture.
//!
//! Controls: N=1024, SAMPLES=7, ITERATIONS=100; ONLY filters comma-separated metrics.
//! N must be at least five so each
//! node has four distinct outgoing edges. Every node has a 128-byte `payload`
//! string and an i64 `value`; every edge has an integer `weight`.
//!
//! Each iteration prepares its receiver before starting an individual timer.
//! The timer covers the public API call and black_box of its result. Receiver
//! setup, returned-value destruction, and receiver destruction are excluded for
//! every metric. Summed call durations are divided by ITERATIONS. Timer overhead
//! remains in the result, which matters for very short duplicate applications.
//! `bytes` is the encoded input/output message size, not heap usage; checkpoints
//! have no wire encoding here and report zero. A `median` row follows each metric.
//!
//! `--memory` holds one graph and CHECKPOINTS checkpoints, default zero. N defaults
//! to 10000 in this mode and cannot be smaller. Use an external OS RSS tool such
//! as `/usr/bin/time -l` on macOS; this program only prints retained record counts.
use std::{
    env,
    hint::black_box,
    time::{Duration, Instant},
};
use zergraph::{Checkpoint, Delta, EdgeKey, Graph, Snapshot, Value};

fn setting(name: &str, default: usize, minimum: usize) -> usize {
    let value = match env::var(name) {
        Ok(value) => value
            .parse::<usize>()
            .unwrap_or_else(|_| panic!("{name} must be an integer, got {value:?}")),
        Err(env::VarError::NotPresent) => default,
        Err(error) => panic!("cannot read {name}: {error}"),
    };
    assert!(value >= minimum, "{name} must be at least {minimum}");
    value
}

fn graph(nodes: usize) -> Graph {
    assert!(nodes >= 5);
    let mut graph = Graph::new();
    let payload = "x".repeat(128);
    for index in 0..nodes {
        let id = format!("node:{index:06}");
        graph.add_node(&id).unwrap();
        graph
            .set_node_property(&id, "payload", payload.clone())
            .unwrap();
        graph.set_node_property(&id, "value", index as i64).unwrap();
    }
    for source in 0..nodes {
        for offset in 1..=4 {
            let target = (source + offset) % nodes;
            let edge = EdgeKey::new(
                format!("node:{source:06}"),
                format!("relation:{offset}"),
                format!("node:{target:06}"),
            );
            graph.add_edge(edge.clone()).unwrap();
            graph
                .set_edge_property(&edge, "weight", offset as i64)
                .unwrap();
        }
    }
    graph
}

struct Fixture {
    base: Graph,
    source: Graph,
    checkpoint: Checkpoint,
    delta: Delta,
    delta_bytes: Vec<u8>,
    snapshot: Snapshot,
    snapshot_bytes: usize,
    empty_delta_bytes: usize,
}

impl Fixture {
    fn new(nodes: usize) -> Self {
        let base = graph(nodes);
        let checkpoint = base.checkpoint();
        let empty_delta_bytes = base.delta_since(&checkpoint).to_bytes().unwrap().len();
        let mut source = base.fork();
        source
            .set_node_property("node:000000", "value", -1_i64)
            .unwrap();
        let delta = source.delta_since(&checkpoint);
        let delta_bytes = delta.to_bytes().unwrap();
        let snapshot = source.snapshot();
        let snapshot_bytes = snapshot.to_bytes().unwrap().len();
        Self {
            base,
            source,
            checkpoint,
            delta,
            delta_bytes,
            snapshot,
            snapshot_bytes,
            empty_delta_bytes,
        }
    }
}

fn smoke() {
    let fixture = Fixture::new(8);
    assert_eq!(fixture.base.nodes().count(), 8);
    assert_eq!(fixture.base.edges().count(), 32);
    assert!(fixture.base.delta_since(&fixture.checkpoint).is_empty());
    assert!(!fixture.delta.is_empty());
    let decoded = Delta::from_bytes(&fixture.delta_bytes).unwrap();
    assert_eq!(decoded, fixture.delta);

    let mut receiver = fixture.base.fork();
    assert!(receiver.apply_delta(&decoded).unwrap());
    assert_eq!(receiver.snapshot(), fixture.snapshot);
    assert!(!receiver.apply_delta(&decoded).unwrap());
    assert_eq!(
        receiver.node("node:000000").unwrap().property("value"),
        Some(&Value::from(-1_i64))
    );
    assert!(receiver.delta_since(&receiver.checkpoint()).is_empty());

    let mut reported = fixture.base.fork();
    let changes = reported.apply_delta_with_changes(&decoded).unwrap();
    assert_eq!(changes.nodes, vec!["node:000000"]);
    assert!(changes.edges.is_empty());
    assert_eq!(reported.snapshot(), fixture.snapshot);
    assert!(reported
        .apply_delta_with_changes(&decoded)
        .unwrap()
        .is_empty());

    let mut full = fixture.base.fork();
    assert!(full.merge(&fixture.snapshot).unwrap());
    assert_eq!(full.snapshot(), fixture.snapshot);
    let mut full_reported = fixture.base.fork();
    assert_eq!(
        full_reported.merge_with_changes(&fixture.snapshot).unwrap(),
        changes
    );
    assert_eq!(full_reported.snapshot(), fixture.snapshot);
    let bytes = fixture.snapshot.to_bytes().unwrap();
    assert_eq!(Snapshot::from_bytes(&bytes).unwrap(), fixture.snapshot);
    println!("smoke\t-\t0\t{}", fixture.delta_bytes.len());
}

struct Run {
    samples: usize,
    iterations: usize,
}

impl Run {
    fn metric<S, R>(
        &self,
        name: &str,
        bytes: usize,
        mut setup: impl FnMut() -> S,
        mut operation: impl FnMut(&mut S) -> R,
    ) {
        if env::var("ONLY").is_ok_and(|names| !names.split(',').any(|item| item == name)) {
            return;
        }
        let mut timings = Vec::with_capacity(self.samples);
        for sample in 0..self.samples {
            let mut elapsed = Duration::ZERO;
            for _ in 0..self.iterations {
                let mut state = setup();
                let started = Instant::now();
                let result = black_box(operation(black_box(&mut state)));
                elapsed += started.elapsed();
                // Both allocations are destroyed only after elapsed is captured.
                black_box(result);
                black_box(state);
            }
            let ns_per_op = elapsed.as_nanos() as f64 / self.iterations as f64;
            timings.push(ns_per_op);
            println!("{name}\t{sample}\t{ns_per_op:.3}\t{bytes}");
        }
        timings.sort_by(f64::total_cmp);
        let middle = timings.len() / 2;
        let median = if timings.len().is_multiple_of(2) {
            (timings[middle - 1] + timings[middle]) / 2.0
        } else {
            timings[middle]
        };
        println!("{name}\tmedian\t{median:.3}\t{bytes}");
    }
}

fn main() {
    if env::args().any(|argument| argument == "--memory") {
        let graph = graph(setting("N", 10_000, 10_000));
        let checkpoints: Vec<_> = (0..setting("CHECKPOINTS", 0, 0))
            .map(|_| graph.checkpoint())
            .collect();
        println!(
            "nodes={} edges={} checkpoints={}",
            graph.nodes().count(),
            graph.edges().count(),
            checkpoints.len()
        );
        black_box((&graph, &checkpoints));
        return;
    }

    println!("metric\tsample\tns_per_op\tbytes");
    if !env::args().any(|argument| argument == "--measure") {
        smoke();
        return;
    }

    let fixture = Fixture::new(setting("N", 1_024, 5));
    let run = Run {
        samples: setting("SAMPLES", 7, 1),
        iterations: setting("ITERATIONS", 100, 1),
    };
    let delta_bytes = fixture.delta_bytes.len();
    run.metric("checkpoint", 0, || (), |_| fixture.base.checkpoint());
    run.metric(
        "checkpoint_capped",
        0,
        || (),
        |_| fixture.base.checkpoint_with_limit(usize::MAX).unwrap(),
    );
    run.metric(
        "checkpoint_rejected",
        0,
        || (),
        |_| fixture.base.checkpoint_with_limit(0),
    );
    run.metric(
        "delta_unchanged",
        fixture.empty_delta_bytes,
        || (),
        |_| fixture.base.delta_since(black_box(&fixture.checkpoint)),
    );
    run.metric(
        "delta_one_property",
        delta_bytes,
        || (),
        |_| fixture.source.delta_since(black_box(&fixture.checkpoint)),
    );
    run.metric(
        "delta_encode",
        delta_bytes,
        || (),
        |_| black_box(&fixture.delta).to_bytes().unwrap(),
    );
    run.metric(
        "delta_decode",
        delta_bytes,
        || (),
        |_| Delta::from_bytes(black_box(&fixture.delta_bytes)).unwrap(),
    );
    run.metric(
        "apply_delta_changed",
        delta_bytes,
        || fixture.base.fork(),
        |receiver| receiver.apply_delta(black_box(&fixture.delta)).unwrap(),
    );
    run.metric(
        "apply_delta_duplicate",
        delta_bytes,
        || fixture.source.fork(),
        |receiver| receiver.apply_delta(black_box(&fixture.delta)).unwrap(),
    );
    run.metric(
        "apply_delta_reported",
        delta_bytes,
        || fixture.base.fork(),
        |receiver| {
            receiver
                .apply_delta_with_changes(black_box(&fixture.delta))
                .unwrap()
        },
    );
    run.metric(
        "full_merge_changed",
        fixture.snapshot_bytes,
        || fixture.base.fork(),
        |receiver| receiver.merge(black_box(&fixture.snapshot)).unwrap(),
    );
    run.metric(
        "full_merge_reported",
        fixture.snapshot_bytes,
        || fixture.base.fork(),
        |receiver| {
            receiver
                .merge_with_changes(black_box(&fixture.snapshot))
                .unwrap()
        },
    );
    run.metric(
        "full_snapshot_encode",
        fixture.snapshot_bytes,
        || (),
        |_| black_box(&fixture.snapshot).to_bytes().unwrap(),
    );
}
