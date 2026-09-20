# Provenance and consolidation

This private library branch consolidates the useful graph concept into `bbopen/zergraph` while preserving the larger projects in place.

Inspected sources:

- `bbopen/zerontology` at `69818dd3e4289960e358f9d46df20c23ff194e50`: owning property graph, timestamp/replica LWW maps, node/edge lifecycle, and graph/store work.
- `bbopen/zergraph_dev` at `e2e948371e064cdbe58178530a4d52b52b2d2523`: component CRDTs, multi-value properties, deletion tracking, and fabric experiments.
- `bbopen/zergraph` at `e699848c793f6ae3f0d0ea089414587dbbb3e429`: original public module scaffold on the private repository's main branch.

The compact implementation retains the LWW graph approach rather than importing either platform wholesale. It simplifies entity state into independent membership/property registers, uses structured labeled edge keys, derives adjacency from visible state, and persists complete snapshots. Fresh writer identities replace cloneable mutable replicas. Owned JSON values replace ontology-specific categories and borrowed byte wrappers. This is a new small API with intentional breaking changes from the scaffold; it is not a drop-in compatibility layer or a source-preserving extraction.

The incomplete event-log persistence boundary is replaced with a complete snapshot boundary. Tests specifically cover defects found during review: repeated insertion damage, ambiguous colon keys, writer-identity collisions, disappearing property changes, stale deletion replay, and dangling edges. A float serialization defect discovered during independent review is covered by an explicit regression and generated cases.

Not imported: ontology schema/language, API/UI, native storage backends, ingestion, auth, telemetry, deployment tiers, routing weights, adaptive sharding, generic CRDT family wrappers, and network transport. Those concepts remain in their original repositories for later consideration.

Prior art was examined to avoid claiming novelty: [rust-crdt](https://github.com/rust-crdt/rust-crdt), [Silk](https://github.com/Kieleth/silk-graph), [crdt-graph](https://github.com/bkbkb-net/crdt-graph), [RoboComp CORTEX](https://github.com/robocomp/cortex), [Automerge](https://github.com/automerge/automerge), and [Graphiti](https://github.com/getzep/graphiti). These are comparisons, not dependencies or copied implementations. The intended value is a small, understandable graph contract.
