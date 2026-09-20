# Merge and lifecycle contract

## State and join

Each node ID and edge key maps to an entity with one timestamped boolean membership register and a map of independent timestamped property registers. A property is either a JSON value or an explicit removal marker. All collections are private ordered maps.

A stamp is `(counter: u64, writer: UUID)`, ordered lexicographically. Every successful write takes the next logical counter. Writer IDs are freshly generated on graph construction, forking, and restore. Receiving a snapshot advances the clock to at least the maximum received counter. Counter exhaustion is an error before mutation.

Merge joins matching registers by maximum stamp and unions their keys. For states produced by independent writers, each register's join is associative, commutative, and idempotent; the pointwise graph join inherits those properties. This argument assumes noncolliding writer UUIDs and faithful snapshots. Runtime tests exercise generated reachable states; they are not a formal verification of all implementations or environments.

If equal stamps on the same register carry unequal values, merge rejects the input before changing any state. Validation does not prove that a sender is honest or reconstruct omitted history. The caller chooses and authenticates peers.

## Graph interpretation

A node is visible when its membership value is true. An edge is visible only when its own membership and both endpoint memberships are true. Hidden edge state is retained and merged normally. Outgoing traversal uses the ordered source range; a derived incoming index stores all retained edge identities, including hidden/deleted ones. Both apply the same visibility predicate as edge lookup. New identities update the index only after merge validation; restore rebuilds it. The index is excluded from snapshot state and wire encoding.

Removing a node changes its membership only. It does not rewrite every incident edge. This means concurrent incident-edge creation cannot expose a dangling edge, and same-ID revival restores still-live relationships. Explicitly removing an edge while an endpoint is hidden prevents that edge's revival. Properties do not modify entity membership.

Reusing an ID means reviving the same entity. Use a fresh ID to model replacement, a new incarnation, or an independent assertion. The core deliberately does not infer identity from names, labels, payloads, or content hashes.

Repeated live insertion is a no-op. Removing unknown/already-removed membership or an absent property is a no-op. Property setters always record a fresh write, even for equal values, because an explicit reassertion can matter when concurrent writes arrive later. Missing endpoints or property targets return typed errors.

The graph permits cycles and self-loops. Edge identity includes source, label, and target, so it supports multiple labels between two nodes but not multiple independently identified occurrences of the same triple. Model occurrences as nodes if each needs its own metadata and lifetime.

## Snapshots

`Snapshot` contains complete state, not just the visible projection. The writer identity and transient clock of the receiving `Graph` are excluded: a restored writer is fresh and initializes its clock from retained stamps.

Version 1 encodes a JSON object containing `version`, sorted `nodes` pairs, and sorted `edges` pairs. Entity state includes membership, properties, and their stamps. Edge keys are structured records, never delimiter-concatenated strings. Node/property keys and edge tuples have deterministic ordering. Nested JSON objects are normalized at setters and decoding, even if a downstream dependency enables `serde_json/preserve_order`. The encoder borrows these immutable ordered values without cloning the graph. The wire shape is unchanged.

Decoding rejects malformed data, unsupported versions, duplicate node/edge identities, zero/invalid writer stamps, and edges without endpoint records. Deleted endpoint records are valid. Snapshot files have no built-in checksum, signature, encryption, compression, framing, atomic replacement, or resource quota. Those belong at the caller's transport/storage boundary. Do not depend on undocumented JSON layout; use the codec API. Cross-version migration and byte-level cryptographic canonicalization are not promised by this preview.

Complete snapshots preserve property deletions and hidden membership. Do not strip records or tombstones before merging: that would change the state contract. There is no automatic reclamation or peer membership protocol. LWW registers retain one value per entity/property key, not a full write history.

## Property meaning

Properties use `serde_json::Value`, including arrays and objects. JSON null is distinct from a removed property. Nested objects are atomic property values: independent keys *inside* one object do not merge separately. Use separate top-level properties when their updates should be independent.

LWW picks a deterministic value; it does not reconcile conflicting factual claims. For example, create separate observation nodes `observation:camera:17` and `observation:person:42`, each linked to an asset and its evidence. The graph then retains both observations without introducing a custom conflict-policy framework.
