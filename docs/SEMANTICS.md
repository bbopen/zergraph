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
UUID writer identities. A live `Graph` does not implement `Clone`. A `Snapshot` does.

Each node and edge has one membership register and a map of property registers.
Membership is a boolean. A property register contains either a JSON value or a
removal marker. Each register also has a stamp.

A stamp is `(counter: u64, writer: UUID)`. The counter orders writes first. The UUID
breaks ties. Each write takes the next counter value. A merge advances the receiver's
clock to at least the largest incoming counter. Counter exhaustion fails before mutation.

The counter represents write order, not the time of an observation.

## Merge

Merge takes the union of register keys and selects the value with the largest stamp
for each key. Membership and properties merge independently. Concurrent writes to
different properties survive. Concurrent writes to the same property select one winner.

For states from independent writers, this join is associative, commutative, and
idempotent. The same properties hold for the graph join. This assumes noncolliding
writer UUIDs and faithful snapshots. Tests exercise generated histories; they are
not a formal proof of the implementation.

If equal stamps on one register have unequal values, merge returns
`Error::ConflictingStamp` without changing state. Validation does not authenticate
a sender or recover omitted records.

`Graph::merge` returns whether stored state changed, including hidden metadata.
It updates only the receiver. Replicas converge after each receives all writers' changes.

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
The index is excluded from snapshots.

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
are structured records. Node keys, property keys, and edge tuples have deterministic order.

Setters and decoding normalize nested JSON object order, including when a dependency
enables `serde_json/preserve_order`. Encoding borrows stored values without cloning
the graph. Version 1 remains byte-compatible with the original packaged fixture.

Decoding rejects malformed data, unsupported versions, duplicate entity IDs, invalid
or zero writer stamps, and edges without endpoint records. Deleted endpoints are valid.

The codec supplies serialization and structural validation. It does not supply
checksums, signatures, encryption, compression, framing, atomic file replacement,
or resource quotas. The caller handles those functions. The current release does
not promise cross-version migration or cryptographic byte canonicalization.

A filtered graph view is not a complete snapshot. Removing hidden records or deletion
markers from an encoded snapshot changes the merge contract. There is no automatic
reclamation or peer membership protocol. Each register retains one value per key,
not a full history of writes.

## Conflicting observations

LWW selects a value by stamp. It does not decide which observation is correct.
Separate observations such as `observation:camera:17` and `observation:person:42`
remain separate records when each has its own node ID. Each can link to the same
asset and to its own evidence. The [repair example](../examples/repair.rs) uses this model.
