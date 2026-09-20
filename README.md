# Zergraph

**A small Rust property graph for independent agents, robots, and applications.**

Give each writer a local graph. Connect observations, evidence, assets, or dependencies. Work independently, exchange snapshots, and merge into the same state.

**667 lines of core Rust · 3 runtime dependencies · deterministic reads · no background runtime**

Zergraph keeps the graph and its merge rules inside your process. Your application chooses how to store snapshots, move them between peers, and act on their contents. It is a private 0.1 release candidate; see [distribution](#distribution).

[Quick start](#quick-start) · [Cookbook](docs/COOKBOOK.md) · [Performance](#measured-performance) · [Hardware and scale](#from-small-devices-to-large-deployments) · [Choosing a library](#when-to-choose-zergraph)

## Quick start

From a checkout of this release candidate:

```sh
cargo run --locked --example evidence
cargo run --locked --example swarm
```

Use that checkout from a neighboring Rust application:

```toml
[dependencies]
zergraph = { path = "../zergraph" }
```

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

Nodes have caller-chosen string IDs. A directed edge is identified by its complete `(source, label, target)` tuple. Properties are owned JSON values. The same primitives work for a test result, a robot observation, a specimen, or an infrastructure dependency.

## When to choose Zergraph

Choose it when your application needs **relationships plus independently mergeable state**, you are calling from Rust, and each graph can fit in memory and travel as a complete snapshot. You get a small API and explicit lifecycle rules, while keeping your existing storage and transport choices.

The alternatives below address different jobs. This table compares their documented roles; no cross-project speed benchmark was run.

| If the main job is… | Consider | Language / interface |
|---|---|---|
| A compact graph with explicit LWW merging and application-owned snapshot exchange | **Zergraph** | Rust |
| A richer graph CRDT with ontology, operation/delta sync, and persistence | [Silk](https://github.com/Kieleth/silk-graph) | Rust, Python |
| An operation-based two-phase graph CRDT | [crdt-graph](https://github.com/bkbkb-net/crdt-graph) | Rust |
| Local graph algorithms and analysis | [petgraph](https://github.com/petgraph/petgraph), [NetworkX](https://networkx.org/) | Rust, Python |
| Collaborative documents, text, lists, or trees | [Automerge](https://github.com/automerge/automerge), [Loro](https://github.com/loro-dev/loro), [Yjs / Yrs](https://github.com/y-crdt/y-crdt) | Rust and JavaScript ecosystems; other bindings vary |
| A decentralized graph with an existing browser/peer ecosystem | [GUN](https://github.com/amark/gun) | JavaScript |
| Temporal AI context with extraction and retrieval integrations | [Graphiti](https://github.com/getzep/graphiti) | Python |
| Durable local SQL or key-value storage | [SQLite](https://sqlite.org/about.html), [bbolt](https://github.com/etcd-io/bbolt) | C / many bindings, Go |
| An operated graph database with server-side queries | [Dgraph](https://docs.dgraph.io/graphql/), [Neo4j](https://neo4j.com/docs/cypher-manual/25/introduction/cypher-neo4j/) | GraphQL / Cypher-facing services |

For robotics-specific shared-world infrastructure, also compare [RoboComp CORTEX](https://github.com/robocomp/cortex). [The full selection guide](docs/ALTERNATIVES.md) explains the different state models, synchronization boundaries, ecosystem choices, and source references.

## Measured performance

The core uses ordered outgoing ranges, an incoming-edge index, and a merge plan that validates incoming state before applying changed records. Snapshot encoding borrows stored values instead of copying the graph.

Apple M3, release build, **1,024 nodes / 4,096 edges**, fixed degree four; median of three process medians, seven samples per process:

| Operation | Before optimization | Current candidate |
|---|---:|---:|
| Outgoing neighborhood | 446 µs | **0.524 µs** |
| Incoming neighborhood | 451 µs | **0.749 µs** |
| Full snapshot merge with one changed property | 1.054 ms | **0.182 ms** |
| Snapshot encoding | 1.981 ms | **1.245 ms** |

These gains compare Zergraph revisions on the same fixture. The core grew by 72 lines. The index trades extra memory and restore/fork work for fast neighbor reads: a separate **10,000-node / 40,000-edge** graph process used **72.8 MiB peak RSS**, versus 61.0 MiB before indexing. Snapshot size is unchanged; the 1,024-node fixture encodes to about **1.62 MB**.

Small source does not imply constant memory or unlimited graph size. [Performance details](docs/PERFORMANCE.md) include every measured operation, slower paths, build costs, raw comparison data, and reproducible commands.

## From small devices to large deployments

The useful scaling unit is **one bounded graph**: a robot's observations, a work session, a site, a tenant, or an experiment. A larger system runs many such graphs and routes each complete snapshot only to the peers that need that graph.

| Scale | Reasonable target | Status |
|---|---|---|
| Smallest experimental device | **ESP32-S3 with 8 MB PSRAM**, tiny graphs, custom ESP-IDF Rust `std` toolchain | Feasible porting target; unbuilt and untested |
| Small Linux device | **64 MB Cortex-A7 board**, such as Luckfox Pico Mini A | Feasible small-workload experiment; image/ABI and memory must be checked |
| Practical edge computer | **Raspberry Pi Zero 2 W, 512 MB RAM** | Recommended first hardware acceptance target; not yet run |
| Native process | Linux, macOS, Windows | Cross-platform CI; measured performance on Apple M3 |
| Data-center composition | **1,000 workers × 100 independent graphs**; about **102 million aggregate nodes** at the primary fixture's shape | Illustrative architecture, with application-owned routing/storage; not a cluster benchmark |

The [deployment guide](docs/DEPLOYMENT.md) supplies primary hardware/toolchain sources and the cluster's memory and bandwidth arithmetic. Zergraph supplies each in-process graph; the deployment owns partition assignment, delivery, storage, and queries across graphs. The upper example describes many bounded graphs, rather than one automatically sharded global graph.

## Cookbook: small programs, useful relationships

Start with a question you want to answer after independent work has been reconciled:

| Question | Starting point |
|---|---|
| Which test run supports this code change, and which evidence was retracted? | [Coding evidence](examples/evidence.rs) |
| Can an inspector correct a reading while a planner adds follow-up work? | [Field inspection](examples/field_inspection.rs) |
| Can four robots preserve disagreeing observations after disconnection? | [Parallel robot observations](examples/swarm.rs) |
| Which sample, transformation, and configuration produced this result? | [Runnable lineage recipe](examples/lineage.rs) |
| Which salvaged component has evidence that it fits this repair? | [Runnable repair and reuse recipe](examples/repair.rs) |
| Can agents connect archaeological interpretations, biodiversity observations, or anomalies across disciplines? | [Exploratory recipes](docs/COOKBOOK.md) |

The recurring pattern is **separate assertions with linked evidence**. Give each observation or hypothesis its own ID. Two agents can then disagree without overwriting each other's assertion. AI extraction, ranking, and proposed connections belong in the application; the graph makes their relationships portable and inspectable.

[Open the cookbook](docs/COOKBOOK.md) for executable starting points, compact graph models, and speculative applications that use the same API.

## A contract small enough to learn

| Concept | Rule |
|---|---|
| Writers | `new`, `default`, `fork`, and restore create fresh UUID writer identities. A live `Graph` is not cloneable. |
| Merge | Membership and each property are independent LWW registers ordered by `(logical counter, writer UUID)`. Peers converge after exchanging complete state. |
| Conflicts | Different property keys retain independent updates. Concurrent writes to one property select one deterministic winner. Logical order is not observation time or factual truth. |
| Identity | Edge keys are structured tuples; IDs may contain colons, Unicode, or empty strings. An identical edge tuple denotes the same relationship. |
| Deletion | Removing a node hides incident edges. Re-adding the same ID revives retained properties and still-live relationships. Use a fresh ID for a replacement entity. |
| Properties | JSON `null` is a value; deletion has a separate marker. Nested JSON objects are atomic property values. |
| Snapshots | Include hidden records and tombstones. Duplicated and reordered delivery is supported. Deletion does not reclaim or securely erase stored data. |

The API covers insertion/removal, immutable lookup and iteration, incoming/outgoing relationships, per-property edits, snapshots, and merge. It leaves transport, durability, permissions, schema, historical queries, and distributed task ownership to the application. There is no partial-snapshot protocol or tombstone reclamation in this version.

[Precise semantics](docs/SEMANTICS.md) · [Integration guide for coding agents](docs/INTEGRATION.md) · `cargo doc --no-deps --open`

## Read the implementation

| File | Responsibility |
|---|---|
| [`src/lib.rs`](src/lib.rs) | Public graph API, writer identity, visibility, and adjacency |
| [`src/state.rs`](src/state.rs) | LWW records, validation, and merge plans |
| [`src/snapshot.rs`](src/snapshot.rs) | Versioned snapshot codec |

The rest of the repository explains, demonstrates, and verifies that core. [Provenance](docs/PROVENANCE.md) records its relationship to the earlier Zerontology and CRDT fabric experiments; [the changelog](CHANGELOG.md) records the release candidate.

## Develop and verify

```sh
cargo check --lib --locked       # fast edit loop; retain Cargo's build cache
cargo test --all-targets --locked
cargo test --doc --locked
cargo clippy --all-targets --locked -- -D warnings
```

CI also checks the `serde_json/preserve_order` configuration, runs every cookbook example, and verifies the packaged crate on Linux, macOS, and Windows. Generated tests exercise merge laws, writer restarts, deletion/revival, index consistency, floating-point transport, and snapshot compatibility.

Performance measurements are opt-in: `cargo bench --bench perf -- --measure`. Build the harness once, then run the executable Cargo reports with `--measure` to repeat measurements without a build cycle. Ordinary tests use its tiny smoke mode. [Contributor guide](CONTRIBUTING.md).

## Distribution

Private 0.1 release candidate; this is a release pull request, not a public registry release. Use the reviewed checkout as a path dependency or pin an authenticated Git dependency to the revision you reviewed. `publish = false` remains set. [LICENSE](LICENSE) preserves the repository's existing proprietary terms.
