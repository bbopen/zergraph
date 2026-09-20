# Graph semantics

Zergraph uses last-write-wins registers, abbreviated LWW. Each register stores one
value and the stamp that orders its writes.

## Identity and properties

A node has a string ID. A directed edge has a structured `(source, label, target)`
key. IDs and labels accept any UTF-8 string, including colons and empty strings.
The graph permits cycles and self-loops.

Two nodes can have several edges with different labels. An identical triple denotes
one edge. Separate occurrences need separate nodes if each has its own properties
or lifetime.

Properties contain owned `serde_json::Value` values. JSON `null` is a value.
Property deletion has a separate marker. A nested object is one property value;
its internal keys do not merge independently.

## Writers and stamps

`Graph::new`, `Graph::default`, `Graph::fork`, and `Graph::from_snapshot` create fresh
UUID writer identities. A live `Graph` does not implement `Clone`. `Snapshot`,
`Checkpoint`, and `Delta` can be cloned.

Each node and edge has one membership register and a map of property registers.
Membership is a boolean. A property register contains either a JSON value or a
removal marker. Each register also has a stamp.

A stamp is `(counter: u64, writer: UUID)`. The counter orders writes first. The UUID
breaks ties. Each write takes the next counter value. A merge advances the receiver's
clock to at least the largest incoming counter. Counter exhaustion fails before mutation.

The counter represents write order, not the time of an observation.

## Merge and change reports

Merge takes the union of register keys and selects the value with the largest stamp
for each key. Membership and properties merge independently. Concurrent writes to
different properties survive. Concurrent writes to the same property select one winner.

For states from independent writers, this join is associative, commutative, and
idempotent. The same properties hold for the graph join. This assumes noncolliding
writer UUIDs and faithful snapshots or deltas. Tests exercise generated histories;
they are not a formal proof of the implementation.

If equal stamps on one register have unequal values, merging returns an error without
changing state. Validation does not authenticate a sender or recover omitted records.

`Graph::merge` and `Graph::apply_delta` return whether stored state changed,
including hidden metadata. `Graph::merge_with_changes` and
`Graph::apply_delta_with_changes` return `MergeChanges`.

`MergeChanges.nodes` lists node IDs whose retained records changed.
`MergeChanges.edges` lists changed edge records and edges whose visibility may have
changed because an endpoint membership changed. Both lists are sorted and deduplicated.
A duplicate input produces an empty report. An error produces no report and leaves
the graph unchanged. The report identifies work for a view refresh; callers query
the graph to determine final visibility.

## Visibility and deletion

A node is visible when its membership is true. An edge is visible when its membership
and both endpoint memberships are true. Hidden records remain in stored state.

Removing a node changes its membership only. It hides incident edges, including
edges added concurrently. Re-adding the same ID restores retained properties and
still-live edges. Removing an edge while an endpoint is hidden prevents that edge
from returning when the endpoint returns.

An ID always identifies the same entity. A replacement item or independent assertion
needs a new ID. Property updates do not change membership.

Outgoing queries use an ordered source range. The incoming index stores all retained
edge identities, including hidden edges. Both queries apply the same visibility rule
as edge lookup. Merge updates the index after validation. Restore rebuilds it.
The index is excluded from snapshots and checkpoints.

## Writes and no-ops

Adding an already-visible entity makes no change. Removing unknown or already-removed
membership makes no change. Removing an absent property also makes no change.

Property setters record a fresh write even when the value is equal. This lets a
writer reassert a value after earlier edits. Missing endpoints or invisible property
targets return `MissingNode` or `MissingEdge` errors.

## Snapshot format

A snapshot contains complete state, including hidden records and deletion markers.
It excludes the live `Graph`'s separate writer-ID and clock fields. Register stamps
retain their writer UUIDs and counters. Restore creates a fresh writer and sets its
clock from the retained stamps.

Version 1 encodes a JSON object with `version`, sorted `nodes` pairs, and sorted
`edges` pairs. Each entity includes membership, properties, and stamps. Edge keys
are structured records. Node keys, property keys, and edge tuples have deterministic
order.

Setters and decoding normalize nested JSON object order, including when a dependency
enables `serde_json/preserve_order`. Encoding borrows stored values without cloning
the graph. Version 1 remains byte-compatible with the original packaged fixture.

Decoding rejects malformed data, unsupported versions, duplicate entity IDs, invalid
or zero writer stamps, and edges without endpoint records. Deleted endpoints are valid.

The snapshot codec supplies serialization and structural validation. It does not
supply checksums, signatures, encryption, compression, framing, atomic file
replacement, or resource quotas. The caller handles those functions. The current
release does not promise cross-version migration or cryptographic byte canonicalization.

A filtered graph view is not a complete snapshot. Removing hidden records or deletion
markers from an encoded snapshot changes the merge contract. There is no automatic
reclamation or peer membership protocol. Each register retains one value per key,
not a full history of writes.

## Checkpoints and deltas

A `Checkpoint` is an in-memory, payload-free record of the exact stamps of retained
node and edge membership registers and property registers. It is `Clone` and
`Default`; the default contains no retained stamps. Its memory cost is proportional
to retained register count. It has no wire codec and is meaningful only to the
application's synchronization bookkeeping.

`Graph::checkpoint_with_limit(max_registers)` counts retained membership and property
registers, including hidden records and deletion markers. It returns `None` before
copying if the limit is exceeded. An accepted checkpoint is complete, with the same
meaning as `checkpoint()`. Zero accepts only an empty graph. This counts registers,
not bytes; key lengths and allocator overhead still affect RAM use. The application
limits how many checkpoints and pending candidates it retains.

`Graph::delta_since(&checkpoint)` compares that checkpoint with retained graph state
and returns a `Delta`. Generation scans all retained state. Matching entity keys use ordered
iteration; differing keys use tree lookups. Worst-case time is `O(n log n)`. It does not maintain an operation log or make delta generation
constant-time.

A delta selects registers absent from the checkpoint or with a newer stamp. For
each selected entity it includes those property registers and the current membership
register, even if membership is unchanged.
For included edges it also carries the membership records for both endpoints needed
to validate and preserve edge visibility. It therefore preserves tombstones and
hidden state needed for a later merge; it is not a filtered visible view.

`Delta::to_bytes` and `Delta::from_bytes` use version-1 JSON. The envelope has
`"kind": "delta"` to distinguish it from a complete snapshot. Its retained records
are ordered and structurally validated using the same identity, stamp, and endpoint
rules as state exchange. Decoding or conflicting-stamp failure leaves the receiving
graph unchanged.

Deltas merge with the same LWW join as snapshots. Delivery may be delayed, reordered,
or duplicated. A peer with no acknowledged common state can receive either a complete
snapshot or a delta generated from `Checkpoint::default()`. If a peer is restored,
replaced, or its retained state is unknown, a saved checkpoint no longer establishes
common state: bootstrap again.

Acknowledgements, checkpoint persistence, retry scheduling, transport framing, and
durability are outside Zergraph. The library provides no lease, scheduler, or
exclusive-work semantics.

## Conflicting observations

LWW selects a value by stamp. It does not decide which observation is correct.
Separate observations such as `observation:camera:17` and `observation:person:42`
remain separate records when each has its own node ID. Each can link to the same
asset and to its own evidence. The [repair example](../examples/repair.rs) uses this model.
