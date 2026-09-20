# Choosing a library

Zergraph fits Rust applications that need relationships, independent edits, and
snapshot or incremental delta exchange. The application must store the data and move it between
writers. Each graph must fit in memory.

Other projects cover different requirements. Their CRDT models, deletion rules,
and formats are not interchangeable.

| Project | Model and language | Use it for |
|---|---|---|
| [Silk](https://github.com/Kieleth/silk-graph) | Graph CRDT in Rust with a Python API | Typed ontology, an operation log, delta sync, queries, and persistent storage |
| [CORTEX and DSR](https://github.com/robocomp/cortex) | Robotics working-memory graph in C++ with Python integration | Shared geometric and symbolic state, delta CRDTs, Fast RTPS communication, and RoboComp integration |
| [crdt-graph](https://github.com/bkbkb-net/crdt-graph) | Operation-based two-phase graph in Rust | Added and removed vertex and edge sets, with FlatBuffers operations and optional petgraph conversion |
| [petgraph](https://github.com/petgraph/petgraph) | Local graph structures in Rust | Traversal, shortest paths, spanning trees, and graph algorithms |
| [rust-crdt](https://github.com/rust-crdt/rust-crdt) | State-based and operation-based CRDT types in Rust | Building a custom data model from individual CRDT types |
| [Automerge](https://github.com/automerge/automerge) | Document CRDT with a Rust core and language wrappers | JSON-like documents, compact changes, and a sync protocol |
| [Loro](https://github.com/loro-dev/loro) | Document CRDT in Rust with JavaScript, Swift, Python, and community Go bindings | Text, rich text, maps, lists, movable trees, and versioning |
| [Yjs](https://github.com/yjs/yjs) and [Yrs](https://github.com/y-crdt/y-crdt) | Shared document types in JavaScript and a compatible Rust implementation | Editor integration, text, arrays, maps, undo, and Yjs protocol compatibility |
| [GUN](https://github.com/amark/gun) | Decentralized graph engine in JavaScript | Browser applications and peer synchronization with an existing protocol |
| [SQLite](https://sqlite.org/about.html) | Embedded SQL database in C with many language bindings | Local transactions, durable storage, and SQL queries |
| [bbolt](https://github.com/etcd-io/bbolt) | Embedded key-value database in Go | Durable local key-value storage and transactions |
| [NetworkX](https://networkx.org/) | Graph analysis in Python | Local graph algorithms and analysis |
| [Graphiti](https://github.com/getzep/graphiti) | Agent context framework in Python | Temporal context, extraction, and retrieval through a graph backend and model services |
| [Dgraph](https://docs.dgraph.io/graphql/) and [Neo4j](https://neo4j.com/docs/cypher-manual/25/introduction/cypher-neo4j/) | Graph database services with GraphQL or Cypher interfaces | Server queries, database administration, and shared service access |

## What Zergraph keeps small

Zergraph has one node type, one edge key, JSON properties, and explicit snapshot
and delta operations. The application chooses the meaning of each ID, label, and property.
This avoids adding a schema system or domain framework to the crate.

That choice has costs. Complete snapshots include deleted records. Per-peer
checkpoints retain register stamps, and delta generation scans retained state.
Deltas reduce the transferred data when few registers change. Queries cover lookup and iteration, including
incoming and outgoing edges. A document CRDT is a better starting point for text
editing; a graph database is a better starting point for server queries.

Use a map, SQLite, or petgraph when one writer owns the data and merge is unnecessary.
Use Zergraph when independent writers need its graph merge rules and your application
can supply the storage and transport.

## Sources

The table links to primary repositories and official documentation checked on
2026-09-19. Check each project's current license and release status before adoption.
This guide compares documented capabilities. It contains no performance comparison
between projects.
