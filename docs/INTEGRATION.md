# Integrate Zergraph

Start with the [quick start](../README.md#quick-start). Use this guide to add graph
identity, storage, complete bootstrap, and optional sparse-delta exchange to an
application.

## Choose the data for one graph

Keep each graph small enough to fit in memory. One robot mission, investigation,
project session, or experiment is a useful starting point. Store large payloads
elsewhere. Put their IDs, hashes, or URIs in properties.

Include the graph ID in every storage record or message. Neither snapshots nor
deltas include a tenant or routing ID. Send data only to the matching graph and
retain hidden records and deletion markers.

## Assign stable IDs

Use existing IDs, such as `asset:pump-4`, UUIDs, or URIs. Reuse an ID for the same
entity. Allocate a new ID for a replacement.

An edge's identity is `(source, label, target)`. If several occurrences need their
own properties or lifetimes, create a node for each occurrence.

Give competing observations separate nodes with author, source, and time properties.
Keep independently editable facts under separate property keys. A nested JSON object
is one value; its internal keys do not merge independently.

## Create independent writers

| Operation | Result |
|---|---|
| `Graph::new()` | Empty graph with a fresh writer ID |
| `graph.fork()` | Copy of current state with a fresh writer ID |
| `Graph::from_snapshot(snapshot)` | Restored state with a fresh writer ID |
| `graph.snapshot()` | Complete state that can be cloned and shared |
| `graph.checkpoint()` | Payload-free record of retained register stamps |

Let one owner mutate each graph. Move independent graphs onto standard threads when
needed, as the [swarm example](../examples/swarm.rs) does. A live `Graph` does not
implement `Clone`.

## Bootstrap a peer with a complete snapshot

A peer with no acknowledged common state usually receives a complete snapshot.

1. Capture `let candidate = graph.checkpoint()` before constructing the snapshot.
2. Capture and encode `graph.snapshot().to_bytes()` without intervening edits.
3. Store or send the bytes with the graph ID using the application's durability and
   transport.
4. Decode with `Snapshot::from_bytes()` and merge or restore it at the peer.
5. Promote the candidate checkpoint for that peer only after the application receives
   an acknowledgement that this exact bootstrap was applied.

Duplicate snapshot delivery is valid. Merge affects only the receiver; applications
must send replies or forward state themselves.

`Checkpoint::default()` contains no retained stamps. A delta generated from it
contains the source's complete retained state and can also bootstrap a peer. A full
snapshot is the usual bootstrap and backup format. In either case, a checkpoint is
not a durable synchronization protocol. A restarted, replaced, or restored peer
does not prove it still has the old state. Treat it as a new peer and establish a
new acknowledgement.

## Send one acknowledged delta at a time

After bootstrap, a sender can avoid resending unchanged register payloads.

```rust
let candidate = source.checkpoint(); // Capture before deriving the delta.
let delta = source.delta_since(&acknowledged);
let bytes = delta.to_bytes()?;

// Send `bytes`. Do not replace `acknowledged` if this attempt is dropped.
let received = zergraph::Delta::from_bytes(&bytes)?;
let changes = destination.apply_delta_with_changes(&received)?;

// Promote only after the application has an acknowledgement for this send.
acknowledged = candidate;
// Empty reports are valid, including retries after an acknowledgement was lost.
# Ok::<(), zergraph::Error>(())
```

Keep one in-flight delta per peer. If delivery fails or acknowledgement is absent,
retry the same encoded batch while retaining the old checkpoint. Do not make a new
delta from the candidate checkpoint until the earlier batch is acknowledged.
The [work-board example](../examples/work_board.rs) makes this sequence explicit.

Checkpoints are payload-free and retain one stamp for each retained membership or
property register. They are memory proportional to retained register count.
`Graph::delta_since(&checkpoint)` currently compares the checkpoint with all retained
state. Worst-case time is `O(n log n)` in retained registers. It does not retain an operation log,
provide peer discovery, negotiate acknowledgements, or schedule retries.

## Receive and inspect a delta

Decode transport bytes with `Delta::from_bytes()`, then choose one receive API:

| Operation | Result |
|---|---|
| `graph.apply_delta(&delta)` | Whether stored state changed |
| `graph.apply_delta_with_changes(&delta)` | `MergeChanges` describing stored record changes |
| `graph.merge_with_changes(&snapshot)` | The same report for a complete snapshot |

A repeated delta returns `false` or an empty `MergeChanges`, respectively. On a
validation or conflicting-stamp error, the graph remains unchanged.

`MergeChanges.nodes` and `MergeChanges.edges` are sorted and deduplicated. They
report retained records that changed. The edge list also includes edges whose
visibility may change when an endpoint's membership changes. Use the report to
refresh a local view, then query the graph for final visibility; it is not a list
of permissions, jobs, leases, or exclusive work assignments.

## Store and protect bytes

For files, use an atomic replacement and flush strategy for the target operating
system. For a database, store graph bytes, graph ID, and the application delivery
record in one transaction. Checkpoints have no persistence codec; establish common
state again after losing them. Zergraph does not write to disk.

Apply authentication, framing, encryption when required, and message-size limits at
the transport boundary. The codec validates its graph structure and version but does
not provide signatures, checksums, compression, resource quotas, or a cross-version
migration protocol.

## Read the result

Use `node`, `edge`, `nodes`, `edges`, `outgoing`, and `incoming`. These methods
return immutable views in deterministic order. Every visible edge has visible endpoints.

Removing a node hides its edges but retains their state. Remove an edge explicitly
if it must stay removed when an endpoint returns. A snapshot or delta still carries
the retained metadata needed to preserve this rule.

## Check your application

Start with two writers that update different facts. Retract an old relationship,
deliver snapshots and deltas in different orders with duplicates, then save and
restore the result. Check that the graph still answers the original question.

Measure retained state, complete snapshot bytes, delta bytes, peak memory during
exchange, and the queries your application uses. See [performance](PERFORMANCE.md),
[deployment](DEPLOYMENT.md), and the [semantics reference](SEMANTICS.md).
