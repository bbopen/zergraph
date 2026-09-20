# Integrating Zergraph

Use the [README](../README.md) for the quick start and [cookbook](COOKBOOK.md) for domain patterns. This page is the compact handoff for a human or coding agent incorporating the library.

## Choose one graph's boundary

Pick a working set whose complete state can live in memory and be exchanged: one robot mission, investigation, project session, experiment, or tenant. Keep large payloads in your existing storage; graph properties can contain artifact IDs, hashes, and URIs.

The application identifies which graph a snapshot belongs to. The codec contains no tenant, authorization, or routing envelope. Put that information in your own message/storage envelope and route a whole snapshot to the correct graph. Never create a partial snapshot by removing hidden records or tombstones from the encoded JSON.

## Give things durable identities

Node IDs belong to your domain. `asset:pump-4`, a UUID, a URI, or a content identifier can all work. Reuse an ID for the same entity; allocate a new ID for its replacement. An edge's identity is `(source, label, target)`. Use an observation or assertion node when multiple independent occurrences of the same relationship must coexist.

Store independently editable facts as separate top-level properties. A nested JSON object is one atomic property value. For competing observations, use separate assertion nodes with author/source/time properties rather than a single shared `status` value. Application timestamps describe the observation; the library's logical clock describes write ordering.

## Start independent writers explicitly

- `Graph::new()` creates an empty writer.
- `graph.fork()` copies current state into a new writer.
- `Graph::from_snapshot(snapshot)` restores state into a new writer.
- `graph.snapshot()` captures immutable, cloneable state for sharing.

UUID writer identity is automatic. A live graph is intentionally not `Clone`. You can move independent graphs onto standard threads, as the [swarm example](../examples/swarm.rs) does. Share immutable snapshots; let one owner mutate each graph, or put synchronization around it in your application when necessary.

## Store and exchange state

1. Capture `graph.snapshot()` and encode it with `to_bytes()`.
2. Use your application's storage mechanism to durably commit those bytes. The library does not save or flush automatically.
3. Transfer the bytes with the graph's routing identity through your existing transport.
4. Decode with `Snapshot::from_bytes()` and merge with `graph.merge(&snapshot)`.
5. Deliver complete state in both directions, or through an application topology that eventually reaches every intended replica.

Duplicate delivery is fine. A merge updates only the receiver, and no background process sends its state onwards. The return value reports changes to stored state, including hidden metadata; it is not an acknowledgment that another peer received anything. The snapshot codec validates structure/version and same-stamp conflict checks protect merge atomicity. The application supplies trust, message-size limits, and transport authentication appropriate to its environment.

For files, choose an atomic replacement and durability strategy for your operating system. A successful `std::fs::write` alone is not a library-provided crash-recovery protocol. For database storage, store the snapshot and routing metadata in the transaction your application already uses.

## Read a consistent graph view

Use `node`, `edge`, `nodes`, `edges`, `outgoing`, and `incoming`; views are immutable and deterministic. Every visible edge has visible endpoints. Removing a node hides its incident relationships, but retains their underlying state. Explicitly remove an edge if it should remain gone after the endpoint returns.

Snapshots still hold deletions and hidden properties. Treat them as retained data when choosing storage policy. Serialization format version 1 is a library interchange format; use the codec rather than editing its JSON representation directly.

## Keep the first application small

A useful acceptance case is: two independent writers update different facts, one retracts an old relationship, the application delivers duplicate/reordered snapshots, and the restored graph answers the original question. The [evidence](../examples/evidence.rs), [inspection](../examples/field_inspection.rs), and [swarm](../examples/swarm.rs) examples are copyable starting points.

Measure retained graph size, encoded snapshot bytes, peak memory during clone/decode/restore, and the queries that actually matter. Then consult [performance](PERFORMANCE.md) and [deployment](DEPLOYMENT.md) before adding indexing, alternate storage, or a new synchronization protocol.
