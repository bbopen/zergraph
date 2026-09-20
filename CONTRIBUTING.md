# Contributing

Zergraph is a small graph library. A contribution should make that graph easier to use, understand, or verify. Domain recipes belong in the cookbook and examples; application storage, transport, planning, and services belong with their applications.

## Work loop

```sh
cargo check --lib --locked
cargo test --test adjacency --locked   # choose the test relevant to your change
```

Keep `target/` between edits. Ordinary tests do not run performance timing loops. Property-test dependencies are development-only; unused subprocess/timeout features are disabled.

## Release checks

```sh
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
cargo test --doc --locked
cargo test --locked --features serde_json/preserve_order
cargo run --locked --example evidence
cargo run --locked --example field_inspection
cargo run --locked --example swarm
cargo run --locked --example lineage
cargo run --locked --example repair
cargo package --locked
```

Run additional cookbook examples whenever they are added; keep the CI workflow aligned. `cargo test --all-targets` compiles examples but does not execute their `main` functions. Package verification runs from a clean checkout; commit intended changes before `cargo package --locked`.

## Change the contract deliberately

Read [SEMANTICS.md](docs/SEMANTICS.md) before changing writers, deletion, visibility, property handling, or merge. Keep failed merges atomic. Fresh writers must not reuse another live writer's identity. Indexes must agree with the visible graph before and after merge/restore. Snapshot tombstones and exact JSON numbers must survive transport.

Add a regression that demonstrates the behavior being repaired. Keep compatibility fixtures when preserving the wire format; document and version intentional wire changes. Update the README and relevant recipes when the public contract changes. Prefer a focused helper or example over a new generic subsystem.

## Measure performance changes

Use `cargo bench --bench perf -- --measure` in a release build. Repeat the same fixture for baseline and candidate, retain raw samples, and report slower paths as well as improvements. Benchmark scripts belong outside the core. Record machine/toolchain, graph shape, property payloads, snapshot sizes, and memory cost. Numbers from one laptop do not establish hardware or cluster capacity.

The detailed [performance report](docs/PERFORMANCE.md) is the model for provenance and tradeoffs. New dependencies or internal worker threads should earn their cost through an actual workload.

## Documentation and release status

Recipe code should execute and answer a concrete question after merge and restore. Mark exploratory ideas as proposals. Cite primary sources for hardware and alternative projects. Keep measured, inferred, and illustrative capacity statements distinct.

This repository currently retains its proprietary license and disables registry publication. A public license, public registry release, or migration commitment is a separate project decision. The [changelog](CHANGELOG.md) describes what the candidate actually contains.
