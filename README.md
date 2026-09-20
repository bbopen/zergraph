# zergraph

A small Rust property graph that independent writers can edit and merge.

Use it to connect evidence, dependencies, assets, observations, and other caller-defined entities. Nodes have string IDs, edges have string labels, and properties are owned JSON values. The library runs in your process; your application saves snapshots and carries them between peers.

This is a **private 0.1 release candidate**. It has three direct runtime dependencies (`serde`, `serde_json`, `uuid`), no native database dependencies, no async runtime, and no background work.

## Try it

From this checkout:

```sh
cargo run --locked --example evidence
cargo run --locked --example field_inspection
cargo test --locked
```

Use it from a neighboring Rust application:

```toml
[dependencies]
zergraph = { path = "../zergraph" }
```

```rust
use zergraph::{EdgeKey, Graph, Snapshot};

fn main() -> Result<(), zergraph::Error> {
    let mut graph = Graph::new();
    graph.add_node("change:42")?;
    graph.add_node("test:17")?;
    let mut peer = graph.fork();

    graph.add_edge(EdgeKey::new("change:42", "validated-by", "test:17"))?;
    peer.set_node_property("test:17", "passed", true)?;
    graph.merge(&peer.snapshot())?;

    for edge in graph.outgoing("change:42") {
        println!("{} -> {}", edge.key().label, edge.key().target);
    }

    let bytes = graph.snapshot().to_bytes()?;
    let restored = Graph::from_snapshot(Snapshot::from_bytes(&bytes)?);
    assert_eq!(restored.edges().count(), 1);
    Ok(())
}
```

`Snapshot` is immutable and cloneable. Every `Graph::new`, `Graph::default`, `fork`, and `from_snapshot` creates a fresh UUID writer identity. A live graph is not cloneable. Two peers converge after each has received the other's complete state; a one-way merge updates only its receiver.

## The contract

| Operation | Behavior |
|---|---|
| `add_node`, `add_edge` | Insert or revive membership. Repeating a live insertion preserves properties and returns `false`. Both edge endpoints must be visible. |
| `remove_node` | Hide the node and its incident edges. Re-adding **the same ID revives** its old properties and still-live edges. Use a fresh ID for a replacement entity. |
| `remove_edge` | Remove that relationship, including when a deleted endpoint currently hides it. Endpoint revival will not restore an explicitly removed edge. |
| `set_*_property`, `remove_*_property` | Change individual properties of visible entities. A repeated set records a fresh write. JSON `null` is a value, distinct from deletion. |
| `node`, `edge`, `nodes`, `edges`, `outgoing`, `incoming` | Immutable views in deterministic order. Hidden entities are absent from every read. |
| `snapshot`, `merge` | Transfer complete state, retaining deletions and hidden data. Duplicate and reordered delivery is supported. |

Node/edge membership and each property are independent **last-write-wins registers** ordered by logical counter, then writer UUID. The clock advances after receiving peer state. This is not wall-clock ordering, an observed-remove set, or an add-wins rule. Concurrent writes to one property have one deterministic winner; updates to different properties are retained.

An edge's identity is the complete `(source, label, target)` tuple. IDs can contain colons, Unicode, or empty strings. Distinct labels create distinct relationships; inserting an identical tuple does not create a parallel occurrence. Represent independent claims or observations as nodes with their own IDs when each needs separate evidence or lifetime. [Semantics and snapshot format](docs/SEMANTICS.md) gives the precise contract.

## Keep the boundary small

The caller owns file I/O, durability, transport, access policy, and any action triggered by graph data. To persist state, write `snapshot().to_bytes()` using your application's normal durable file/storage mechanism; restore with `Snapshot::from_bytes`. There is no automatic save or `flush` operation.

Snapshots contain the **whole graph**, including tombstones and hidden properties. Deletion hides data; it does not securely erase it. State grows with distinct entities and property names. Repeated writes to an existing register replace its stored value. This version has no history log, tombstone reclamation, partial replication, schema language, query language, or server.

This is an in-memory library for application-controlled peers. Snapshots have structural/version validation and conflicting-stamp detection, not authentication or Byzantine guarantees. LWW convergence does not guarantee exclusive task claims, a cycle-free dependency graph, or the truth of an observation.

Adjacency reads scan stored edges, with endpoint lookups in ordered maps. Full-state merge and snapshots also touch retained state. No scale, real-time, embedded-hardware, or cross-language wire-compatibility claim is made; measure your workload before adding indexing or delta sync.

## Examples and applications

- [`evidence`](examples/evidence.rs): two reviewers contribute evidence, retract a stale relationship, merge, and restore a snapshot.
- [`field_inspection`](examples/field_inspection.rs): technicians independently correct an observation and schedule follow-up, preserving properties and deletion state.
- [Applications](docs/APPLICATIONS.md): agent evidence, dependency handoffs, disconnected inspection, specimen lineage, and speculative AI-assisted discovery workflows using the same API.

## Development

```sh
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
cargo test --doc --locked
cargo test --locked --features serde_json/preserve_order
cargo package --locked
```

Tests exercise generated three-writer histories, merge laws, writer restart, deletion/revival, scalar and nested JSON transport, and malformed snapshots. Floating-point round-trip support is enabled deliberately: changing a stored value during decoding would invalidate its original CRDT stamp.

The source is a focused consolidation of the graph work in `zerontology` and `zergraph_dev`. The ontology, language, services, storage engines, and sharding experiments remain in their original repositories. See [provenance and scope](docs/PROVENANCE.md).

## Distribution

Private and proprietary, consistent with this repository's existing README policy. `publish = false` prevents accidental registry publication. This change does not select a public license or publish a public release. See [LICENSE](LICENSE).
