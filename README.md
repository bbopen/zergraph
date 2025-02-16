# zergraph: Adaptive Distributed Graph Database

**Embrace Chaos. Grow Graphs.**

zergraph is a novel graph database designed for the challenges of modern, distributed, and dynamic data environments. Built from the ground up in Rust, zergraph prioritizes **scale, resilience, and emergent optimization** over strict ACID guarantees, making it ideally suited for:

* **IoT and Edge Computing:** Swarm robotics, sensor networks, edge analytics.
* **Social Networks:** Friend-of-friend recommendations, dynamic social graphs.
* **Globally Distributed Data:** Knowledge graphs, asset tracking, decentralized data management.

**Key Features (Initial Stage):**

* **CRDT-First Architecture:** Built on Conflict-Free Replicated Data Types (CRDTs) for inherent eventual consistency and automatic conflict resolution.
* **Layered Deployment Model:** Designed to span from resource-constrained embedded devices to scalable cloud infrastructure.
* **Evolvable Edges:** Supports edge deletion and graph evolution through Observed-Remove Sets (OR-Sets).
* **Hybrid Storage:** Optimized storage strategies for embedded (LRU cache + flash overflow) and edge/cloud (B-tree + Bloom filter indexes) layers.
* **libp2p Network Layer:** Utilizes `libp2p` for robust peer-to-peer networking, gossip-based data propagation, and dynamic sharding.
* **Basic Cypher-Like Querying:** Initial support for `MATCH-WHERE-RETURN` queries.

**Architecture - Layered Approach:**

zergraph is structured into three distinct but interconnected layers:

* **Embedded Layer:** Runs on microcontrollers, focused on data collection and efficient resource usage.
* **Edge Layer:** Deployed on edge gateways, responsible for local aggregation, data federation, and optimized query routing.
* **Cloud Layer:** Scalable cloud infrastructure for global data persistence, complex analytics, and system-wide management.

**Getting Started (Early Alpha):**

This is an early alpha version. Currently, you can:

1. **Clone the repository:** `git clone [repository URL]`
2. **Build the project:** `cargo build`
3. **Run basic tests (if implemented in initial commit):** `cargo test`

**Project Status:**

zergraph is currently in **early alpha development**. The initial commit lays the foundation for the core architecture and component modules. Significant development is ongoing. Expect rapid changes and API evolution.

**Contributing:**

TBD

**License:**

Proprietary. Not open source. Not for public use. All rights reserved @bbopen