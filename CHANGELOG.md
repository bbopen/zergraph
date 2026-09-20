# Changelog

## 0.1.0 — private release candidate

- Add a documentation-first README, nine cookbook recipes, and runnable lineage/repair examples.
- Document a sourced hardware envelope and a worked example of application-level cluster composition.
- Compare graph, CRDT, database, and robotics alternatives in Rust, Python, JavaScript, Go, C/C++, and service ecosystems.

- Add measured outgoing ranges, an incoming-edge identity index, and merge plans that apply only changed records after complete validation.
- Encode snapshots from borrowed state; normalize JSON values once at ingress.
- Add an opt-in dependency-free benchmark harness and a parallel four-robot reconciliation example.
- Remove unused property-test fork/timeout dependencies and duplicate feature-branch CI runs.

- Replace the architecture scaffold with one owned property graph library.
- Add labeled structured edges, deterministic views, LWW membership/properties, and explicit node revival.
- Give each new/forked/restored graph a fresh writer identity.
- Add complete versioned snapshots with deletion state and exact floating-point round trips.
- Add independent merge-law, transport, malformed-snapshot, and lifecycle tests plus two executable examples.
- Remove placeholder network, query, storage-tier, security, recovery, and deployment APIs.
- Align private distribution metadata and disable registry publication.

This is a breaking replacement for the earlier placeholder API. No public release or migration from the experimental repositories is implied.
