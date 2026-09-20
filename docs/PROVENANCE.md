# Earlier implementations

The graph library lives in `bbopen/zergraph`. The larger experimental
repositories remain unchanged.

The review used these revisions:

| Repository | Revision | Relevant work |
|---|---|---|
| `bbopen/zerontology` | `69818dd3e4289960e358f9d46df20c23ff194e50` | Property graph, LWW maps, entity lifecycle, and graph storage |
| `bbopen/zergraph_dev` | `e2e948371e064cdbe58178530a4d52b52b2d2523` | Component CRDTs, multi-value properties, deletion tracking, and fabric experiments |
| `bbopen/zergraph` | `e699848c793f6ae3f0d0ea089414587dbbb3e429` | Original module skeleton |

## What this library retains

The implementation retains the LWW graph approach. Each entity has independent
membership and property registers. Edges use structured keys. Snapshots store complete
state. New, forked, and restored graphs get fresh writer identities.

Owned JSON properties replace ontology-specific categories and borrowed byte wrappers.
The API replaces the original skeleton. It is a new implementation, not a compatible
extraction of either experimental library.

## What the review changed

Complete snapshots replace the earlier graph-event persistence path, which did not
reconstruct full graph state. Tests cover repeated insertion, ambiguous colon keys,
writer-ID reuse, missing property changes,
stale deletion replay, and dangling edges. A float serialization regression and
generated numeric cases check exact transport.

Ontology, ingestion, user interfaces, storage backends, authentication, network
transport, and deployment services remain outside the crate. Those experiments are
still available in their original repositories.

The incremental-sync review checked the same revisions. `zergraph_dev` merges
whole node and fabric state. `zerontology` graph events have no original writer
stamps, and replay assigns new local stamps. Neither supplies the delta protocol
used here. Zergraph deltas preserve the stamps of selected registers and compare
exact per-register checkpoints.

The [alternatives guide](ALTERNATIVES.md) lists the projects used for comparison.
Those projects are not dependencies or copied implementations.
