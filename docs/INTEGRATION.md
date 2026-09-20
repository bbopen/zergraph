# Integrate Zergraph

Start with the [quick start](../README.md#quick-start). Use this guide to add graph
identity, storage, and snapshot exchange to an application.

## Choose the data for one graph

Keep each graph small enough to fit in memory and exchange as a complete snapshot.
One robot mission, investigation, project session, or experiment is a useful starting
point. Store large payloads elsewhere. Put their IDs, hashes, or URIs in properties.

Include the graph ID in your storage record or network message. The snapshot codec
does not include a tenant or routing ID. Send each snapshot to the matching graph.
Preserve the complete snapshot, including hidden records and deletion markers.

## Assign stable IDs

Use your existing IDs, such as `asset:pump-4`, UUIDs, or URIs. Reuse an ID for the
same entity. Allocate a new ID for a replacement.

An edge's identity is `(source, label, target)`. If several occurrences need their
own properties or lifetimes, create a node for each occurrence.

Give competing observations separate nodes with author, source, and time properties.
Keep independently editable facts under separate property keys. A nested JSON
object is one value; its internal keys do not merge independently.

## Create independent writers

| Operation | Result |
|---|---|
| `Graph::new()` | Empty graph with a fresh writer ID |
| `graph.fork()` | Copy of the current state with a fresh writer ID |
| `Graph::from_snapshot(snapshot)` | Restored state with a fresh writer ID |
| `graph.snapshot()` | Complete state that can be cloned and shared |

Let one owner mutate each graph. Move independent graphs onto standard threads when
needed, as the [swarm example](../examples/swarm.rs) does. A live `Graph` does not
implement `Clone`.

## Store and exchange snapshots

1. Capture the state with `graph.snapshot()`.
2. Encode the snapshot with `to_bytes()`.
3. Store the bytes and graph ID with your application's durability mechanism.
4. Send the bytes and graph ID through your transport.
5. Decode the received bytes with `Snapshot::from_bytes()`.
6. Merge the snapshot into the matching graph with `graph.merge(&snapshot)`.
7. Deliver each writer's changes to every intended replica.

Duplicate delivery is valid. A merge changes only the receiver. Your application
must send replies or forward state. The merge result says whether stored state
changed, including hidden metadata.

For files, use an atomic replacement and flush strategy for your operating system.
For a database, store the snapshot and graph ID in one transaction. Zergraph does
not write to disk. Apply authentication and message-size limits at your transport.

## Read the result

Use `node`, `edge`, `nodes`, `edges`, `outgoing`, and `incoming`. These methods return
immutable views in deterministic order. Every visible edge has visible endpoints.

Removing a node hides its edges but retains their state. Remove an edge explicitly
if it must stay removed when an endpoint returns. A snapshot still contains hidden
properties and deletion markers.

## Check your application

Start with two writers that update different facts. Retract an old relationship,
deliver snapshots in different orders with duplicates, then save and restore the
result. Check that the graph still answers the original question.

Measure retained state, snapshot bytes, peak memory during exchange, and the queries
your application uses. See [performance](PERFORMANCE.md), [deployment](DEPLOYMENT.md),
and the [semantics reference](SEMANTICS.md).
