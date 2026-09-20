# When to choose zergraph or something else

This is a scope guide, not a benchmark or a claim that these systems have the
same conflict rules. `zergraph` is a private Rust 0.1 library: an in-memory
property graph whose node/edge membership and individual JSON properties are
last-write-wins registers. Applications exchange and persist complete,
versioned snapshots; the library supplies neither network transport, durable
storage, authentication, a query language, schemas, nor partial replication.

Choose a different system when one of those omitted responsibilities is the
main requirement. In particular, a CRDT label alone does not establish the
same deletion, causality, conflict, or wire-format semantics.

| Alternative | Language / primitive | Choose it when | Choose zergraph when |
| --- | --- | --- | --- |
| [Silk](https://github.com/Kieleth/silk-graph) | Rust core with Python API; Merkle-DAG/oplog graph CRDT with typed ontology, delta sync, redb storage, queries, subscriptions, and signing controls | You need schema enforcement, durable storage, operation/delta sync, graph algorithms, or a richer replicated-graph system. Its source is under the Functional Source License with an Apache-2.0 change license; assess those terms. | You need a small in-process Rust graph with application-owned snapshot transport/storage, with no schema or operation log. |
| [RoboComp CORTEX / DSR](https://github.com/robocomp/cortex) | C++ DSR core with Python integration; delta-mutator CRDT working-memory graph for RoboComp agents, including typed geometric and symbolic world data, Fast RTPS communication, and GUI/API integration | You are building a data-oriented robotics working memory shared by RoboComp agents, with robot or simulation components and geometric transforms as first-class concerns. | You need a tiny generic Rust graph for application-defined evidence or observations, with no robotics middleware, robot-control surface, or CORTEX deployment integration. |
| [crdt-graph](https://github.com/bkbkb-net/crdt-graph) | Rust; operation-based 2P2P-Graph with added/removed vertex and edge sets, FlatBuffers operations, and optional petgraph conversion | Your model is the cited two-phase graph CRDT and peers can distribute its operations in the required delivery model. It is dual MIT/Apache-2.0. The public repository shows no releases, so evaluate its protocol/API fit directly. | You need per-property JSON values and the release crate’s full-snapshot, LWW membership/property contract rather than an op-based two-phase graph. |
| [petgraph](https://github.com/petgraph/petgraph) | Rust in-memory graph data structures and algorithms | Traversal, shortest paths, spanning trees, graph representations, or DOT interoperability are the primary job; petgraph is MIT/Apache-2.0. | Independent writers must merge graph state. Add petgraph later at an application boundary if algorithms become necessary. |
| [rust-crdt / `crdts`](https://github.com/rust-crdt/rust-crdt) | Rust collection of serializable state- and operation-based CRDT building blocks | You need to compose established CRDT data types into a domain model yourself; it is Apache-2.0. | A small property-graph API with graph visibility and snapshot encoding is the desired boundary, rather than constructing that layer from primitives. |
| [Automerge](https://github.com/automerge/automerge) | Rust core and multi-language wrappers; JSON-like document CRDT, compact change format, and sync protocol | Your state is chiefly a collaboratively edited document or JSON tree and you want its change/sync ecosystem. Automerge is MIT; its repository describes the Rust API as low-level and less documented than its JavaScript API. | Your domain is named entities plus labeled relationships, and full-state snapshot exchange is enough without document-oriented change sync. |
| [Loro](https://github.com/loro-dev/loro) | Rust CRDT library with JS, Swift, Python, and community Go bindings; text, rich text, maps, lists, movable lists, and trees | You need local-first documents, rich text, ordered collections, movable trees, versioning, or cross-language clients. Loro is MIT and its project describes Loro 1.0 as released. | You need a compact Rust-only graph core with explicit string IDs and edges, without a document/container model or cross-language promise. |
| [Yjs](https://github.com/yjs/yjs) / [Yrs](https://github.com/y-crdt/y-crdt) | JavaScript shared-type CRDT / compatible Rust port | Browser/editor integrations, collaborative text, arrays, maps, undo, awareness, or Yjs binary-protocol interoperability are required. Both projects describe Yrs as Yjs-compatible; Yjs is MIT. | Existing clients are Rust callers exchanging complete snapshots and a graph data model is more direct than adapting shared document types. |
| [GUN](https://github.com/amark/gun) | JavaScript decentralized graph synchronization protocol and embedded engine | A browser-oriented, real-time decentralized graph ecosystem with its own peer sync and encryption-oriented tooling is wanted. Its repository lists Zlib/MIT/Apache-2.0 terms. | Your application must keep transport, authorization, durability, and policy under its own Rust control. |
| [SQLite](https://sqlite.org/about.html) / [bbolt](https://github.com/etcd-io/bbolt) | Embedded C SQL database / embedded Go key-value store | The core need is durable ACID local data, SQL queries, or a Go key-value file. SQLite is public domain; bbolt is MIT. Neither is a graph CRDT or supplies multi-writer replica convergence. | Offline peers must merge independently written graph facts before the application decides how and where to persist snapshots. |
| [NetworkX](https://networkx.org/) / [Graphiti](https://github.com/getzep/graphiti) | Python graph-analysis library / Python temporal AI context-graph framework using a supplied graph backend and LLM/embedding services | Choose NetworkX for Python graph analysis (BSD-3-Clause), or Graphiti for temporal agent context, provenance episodes, hybrid retrieval, and its required backend/service integration (Apache-2.0). Neither is a drop-in embedded Rust replica library. | The application already owns facts and wants deterministic, local mergeable graph state without LLM extraction, embeddings, or database service dependencies. |
| [Dgraph](https://docs.dgraph.io/graphql/) / [Neo4j](https://neo4j.com/docs/cypher-manual/25/introduction/cypher-neo4j/) | Server graph databases with GraphQL/Cypher-facing APIs | Central service operation, server-side query/planning, generated GraphQL APIs, or multi-user database administration is the principal requirement. Dgraph’s repositories use Apache and Dgraph Community licenses; Neo4j has Community and commercial Enterprise editions. Neither description implies zergraph’s offline full-snapshot CRDT contract. | The data is application-controlled local replica state and deploying or operating a graph server is out of scope. |

## Practical selection

Pick `zergraph` for a Rust application that needs a small, deterministic
property graph where independently edited **complete snapshots** can converge
under documented LWW rules, and where the application already has a clear
answer for storage, transport, access policy, and any action taken from graph
data.

Pick a graph CRDT engine such as Silk or crdt-graph when its schema, operation
log, delta protocol, or persistence model is actually required. Pick a document
CRDT when the primary data is collaborative text or a JSON/document tree. Pick
a graph library or graph database when traversal/query operations or centralized
service responsibilities dominate. These choices can be combined at explicit
application boundaries, but their states and conflict semantics should not be
treated as wire-compatible without a separately designed adapter.

## Source and status notes

The links above are primary project repositories or official documentation,
checked on 2026-09-19. License and maintenance status can change; follow the
linked project’s license and release materials before adoption. No row infers
the absence of CRDT behavior from a project not presented as a CRDT, and no
row makes a head-to-head performance claim.
