# Zergraph

Zergraph is a Rust property graph that merges changes from independent writers.
Store entities, labeled relationships, and JSON properties. Each writer edits a
local graph and exchanges snapshots or incremental deltas with other writers.

The core has 884 lines in four Rust modules and three direct runtime dependencies.
Your application provides storage and transport. Zergraph starts no background tasks.

## Quick start

From a checkout of this branch, run the evidence example:

```sh
cargo run --locked --example evidence
```

The example merges two writers, restores a snapshot, and checks that an old test
result stays retracted. It prints:

```text
Evidence reconciled: current test run supports the reviewed claim.
Old support remains retracted after snapshot restore and stale merge.
```

To use Zergraph in another Rust crate, add a path dependency to that crate's
`Cargo.toml`. Set the path to your Zergraph checkout:

```toml
[dependencies]
zergraph = { path = "../zergraph" }
```

Put this example in your application's `src/main.rs` and run `cargo run`:

```rust
use zergraph::{EdgeKey, Graph, Snapshot};

fn main() -> Result<(), zergraph::Error> {
    let mut reviewer = Graph::new();
    reviewer.add_node("change:42")?;
    reviewer.add_node("test:17")?;
    let mut tester = reviewer.fork();

    reviewer.add_edge(EdgeKey::new("change:42", "validated-by", "test:17"))?;
    tester.set_node_property("test:17", "passed", true)?;

    // Exchange the states captured after each writer's independent work.
    let review_state = reviewer.snapshot();
    let test_state = tester.snapshot();
    reviewer.merge(&test_state)?;
    tester.merge(&review_state)?;
    assert_eq!(reviewer.snapshot(), tester.snapshot());

    // The application stores or transports these bytes.
    let bytes = reviewer.snapshot().to_bytes()?;
    let restored = Graph::from_snapshot(Snapshot::from_bytes(&bytes)?);
    assert_eq!(restored.outgoing("change:42").count(), 1);
    assert_eq!(restored.node("test:17").unwrap().property("passed"), Some(&true.into()));
    Ok(())
}
```

The program exits without output when both writers have the same state and the
restored graph contains the test result.

## Incremental exchange

After a peer receives a full snapshot, retain the checkpoint that matches that
send. Build later deltas from that acknowledged checkpoint:

```rust
// Capture before building the delta. Do not replace this with a later checkpoint.
let candidate = writer.checkpoint();
let bytes = writer.delta_since(&acknowledged).to_bytes()?;
let changes = peer.apply_delta_with_changes(&zergraph::Delta::from_bytes(&bytes)?)?;
// Promote only after this message was applied and acknowledged.
acknowledged = candidate;
```

The application sends the bytes and acknowledgments. `changes` lists node IDs and
edge keys to reread, including edges affected by node removal or revival. Retries
and reordered delivery are safe. Checkpoints store stamps for every retained
register, and delta generation scans the graph.
`checkpoint_with_limit(max_registers)` returns `None` before copying if the checkpoint
would exceed that count. The [bounded-sync recipe](docs/COOKBOOK.md#bound-retained-synchronization-knowledge)
also caps retained copies, including pending sends. These are configurable counts,
not byte limits. See the
[work-board recipe](docs/COOKBOOK.md#sync-a-work-board-after-bootstrap) for a complete example.

## Documentation

| You want to... | Read |
|---|---|
| Try the API | [Quick start](#quick-start) |
| Build an application | [Cookbook](docs/COOKBOOK.md) and [integration guide](docs/INTEGRATION.md) |
| Look up merge and deletion rules | [Semantics](docs/SEMANTICS.md) or run `cargo doc --no-deps --open` |
| Assess a design | [Alternatives](docs/ALTERNATIVES.md), [deployment](docs/DEPLOYMENT.md), and [application ideas](docs/APPLICATIONS.md) |

## When to choose Zergraph

Use Zergraph when independent writers need to merge relationships and properties,
and each graph fits in memory. Bootstrap and back up with complete snapshots. Use
deltas to send changed registers since an acknowledged checkpoint. Both preserve
deletion metadata. Keep each graph scoped to one mission, project, or experiment.

| Main requirement | Consider |
|---|---|
| An in-process Rust graph with LWW merge, snapshots, and sparse deltas | Zergraph |
| A graph CRDT with schema, delta sync, and persistence | [Silk](https://github.com/Kieleth/silk-graph), Rust and Python |
| An operation-based two-phase graph CRDT | [crdt-graph](https://github.com/bkbkb-net/crdt-graph), Rust |
| Local graph algorithms | [petgraph](https://github.com/petgraph/petgraph), Rust, or [NetworkX](https://networkx.org/), Python |
| Collaborative text, lists, or documents | [Automerge](https://github.com/automerge/automerge), [Loro](https://github.com/loro-dev/loro), or [Yjs and Yrs](https://github.com/y-crdt/y-crdt), Rust and JavaScript ecosystems |
| A decentralized graph with browser and peer tools | [GUN](https://github.com/amark/gun), JavaScript |
| Agent context with extraction and retrieval | [Graphiti](https://github.com/getzep/graphiti), Python |
| Shared robotics world state and middleware | [CORTEX](https://github.com/robocomp/cortex), C++ and Python |
| Durable local SQL or key-value storage | [SQLite](https://sqlite.org/about.html), C, or [bbolt](https://github.com/etcd-io/bbolt), Go |
| Graph queries across a database service | [Dgraph](https://docs.dgraph.io/graphql/) or [Neo4j](https://neo4j.com/docs/cypher-manual/25/introduction/cypher-neo4j/) |

The [selection guide](docs/ALTERNATIVES.md) explains these differences. This is a
comparison of documented features. We have not benchmarked Zergraph against these projects.

## Measured performance

The incremental-sync benchmark uses 1,024 nodes and 4,096 edges on Apple M3.
A one-property edit encodes to 294 bytes, compared with a 1,746,030-byte full
snapshot. Delta generation takes 0.176 ms; capturing a checkpoint takes 0.493 ms.
Generation scans retained state. Checkpoints use memory per peer. See
[Sync performance](docs/SYNC_PERFORMANCE.md) for the fixture, all timings, and
checkpoint memory measurements.

The following table records the earlier snapshot-core optimization. Its fixture
uses different property values from the sync benchmark.

The benchmark used an Apple M3 and a release build with 1,024 nodes and 4,096 edges.
Each node had four outgoing edges. Results are medians of three process medians,
with seven samples per process.

| Operation | Before optimization | Optimized snapshot core |
|---|---:|---:|
| Outgoing neighborhood | 446 µs | 0.524 µs |
| Incoming neighborhood | 451 µs | 0.749 µs |
| Full snapshot merge with one changed property | 1.054 ms | 0.182 ms |
| Snapshot encoding | 1.981 ms | 1.245 ms |

The incoming-edge index speeds up reads but uses more memory and makes restore and
fork slower. A separate process with 10,000 nodes and 40,000 edges used 72.8 MiB peak
RSS, up from 61.0 MiB. The 1,024-node snapshot remains 1,619,049 bytes.

[Performance details](docs/PERFORMANCE.md) include all timings, memory costs, build
times, raw comparison data, and commands to repeat the measurements.

## Hardware and scale

| Target | Status |
|---|---|
| ESP32-S3 with 8 MB PSRAM | Proposed port for small graphs. Requires a custom ESP-IDF Rust `std` toolchain. |
| Luckfox Pico Mini A with 64 MB RAM | Proposed Linux port. The binary must match the board's system image. |
| Raspberry Pi Zero 2 W with 512 MB RAM | Suggested first device test. |
| Linux, macOS, and Windows | CI passes. Performance measurements use Apple M3. |
| 1,000 workers with 100 independent graphs each | Design example with about 102 million total nodes at the benchmark graph size. |

The device ports and cluster example have not been tested. The
[deployment guide](docs/DEPLOYMENT.md) gives hardware sources, memory budgets, and
bandwidth calculations. The application assigns graphs to workers and handles
queries between graphs. Adding workers does not split one graph across the cluster.

## Merge rules

Each node's membership, each edge's membership, and each property has a
last-write-wins register, or LWW register. Writes are ordered by logical counter,
then writer UUID. This order does not represent the time of an observation.

Different property keys merge independently. Concurrent writes to the same key
select one winner. Give each observation its own node ID to preserve disagreement.

Removing a node hides its edges. Re-adding the same ID restores its retained
properties and still-live edges. A replacement entity needs a new ID. Snapshots
retain deleted records; deletion does not reclaim memory.

The [semantics reference](docs/SEMANTICS.md) defines the full contract.

## Development

```sh
cargo check --lib --locked
cargo test --all-targets --locked
cargo test --doc --locked
cargo clippy --all-targets --locked -- -D warnings
```

Keep `target/` between edits. CI also checks `serde_json/preserve_order`, runs all
seven examples, and verifies the package. See [Contributing](CONTRIBUTING.md) for the
release commands and [provenance](docs/PROVENANCE.md) for the earlier implementations.

## Distribution

This is a private 0.1 release candidate. Use a path dependency or an authenticated
Git dependency pinned to the revision you reviewed. Registry publication is disabled
with `publish = false`. The repository retains its [proprietary license](LICENSE).
