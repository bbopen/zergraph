# Changelog

## 0.1.0 private release candidate

- Replace the original module skeleton with a property graph library.
- Document local-crate and module-copy integration for host-maintained source.
- Add labeled edge keys, deterministic views, independent LWW membership and
  properties, and explicit deletion and revival rules.
- Give each new, forked, and restored graph a fresh writer identity.
- Add versioned snapshots that retain deletions and exact JSON numbers.
- Add outgoing range queries, an incoming-edge index, and merge plans that apply
  changed records after complete validation.
- Encode snapshots from borrowed state and normalize JSON object order at input.
- Add tests for merge laws, transport, malformed snapshots, adjacency, and lifecycle.
- Add sparse state deltas with exact per-register checkpoints and acknowledged
  retry guidance. Preserve full version-1 snapshot bytes.
- Add optional merge reports with changed records and affected incident edges.
- Add an optional register-count cap before checkpoint allocation and a bounded-copy
  example. Exceeding a cap defers work; it never truncates synchronization metadata.
- Add a runnable agent work board with independent attempts, evidence, and retry.
- Add five other runnable examples for evidence, inspection, robot observations, lineage,
  and repair.
- Add benchmark results, hardware proposals, a cluster design example, and comparisons
  with libraries in other languages.
- Organize plain-English docs around the quick start, task guides, reference, and
  design explanations. Put unbuilt application ideas on a separate page.
- Add an opt-in benchmark program and remove unused proptest features and duplicate
  feature-branch CI runs.
- Remove placeholder network, query, storage-tier, security, recovery, and deployment APIs.
- Retain the proprietary license and disable registry publication.

The API replaces the earlier skeleton. It does not provide a migration from the
experimental repositories.
