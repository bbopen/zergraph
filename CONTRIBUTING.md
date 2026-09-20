# Contributing

Keep application-specific code in examples and the cookbook. Keep storage, transport,
and services in the application that uses Zergraph. Prefer small host functions to
a configuration framework in the library. [Vendoring](docs/VENDORING.md) explains
how to maintain the source as a local crate or application modules.

## Check an edit

```sh
cargo check --lib --locked
cargo test --test adjacency --locked
```

Choose the integration test that covers your change. Keep `target/` between edits.
Ordinary tests do not run performance timing loops.

## Check a release

Run these checks before finalizing a behavior change:

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
cargo run --locked --example work_board
cargo run --locked --example bounded_sync
```

Commit the intended changes, then verify the package from the clean checkout:

```sh
cargo package --locked
```

Add each new example to CI. `cargo test --all-targets` compiles examples but does not
run their `main` functions. Run the commands above to execute their assertions.

## Change behavior

Read [Graph semantics](docs/SEMANTICS.md) before changing writers, deletion, visibility,
properties, or merge. Add a regression test for the behavior being repaired. Keep
failed merges atomic and indexes consistent with visible state.

Preserve the snapshot fixtures when retaining the format. Document and version an
intentional format change. Update the relevant examples and reference when the
public contract changes.

## Measure performance

Run `cargo bench --locked --bench perf -- --measure` or
`cargo bench --locked --bench sync -- --measure` in a release build. Use the same
fixture for the baseline and candidate. Repeat the runs and retain the raw samples.
Report memory costs and slower operations as well as faster ones. Record the machine,
toolchain, graph shape, properties, and snapshot size.

Follow the method in [Performance](docs/PERFORMANCE.md). Add dependencies or internal
threads only when a measured workload justifies them.

## Edit documentation

Write plain technical English. Use the code's names consistently. State who does
what, put conditions before instructions, and split sentences that carry several ideas.
Cut promotional claims, repeated caveats, and unexplained jargon.

Keep the quick start runnable. Put task steps in the cookbook and integration guide,
exact behavior in the semantics reference, and design discussion in explanation pages.
Link between them where needed. Keep unbuilt application ideas separate from examples
that run. Use [Diátaxis](https://diataxis.fr/start-here/) to choose each page's purpose.

Check changed code examples and relative links. Preserve benchmark values and their
measurement conditions. Label proposed hardware targets and cluster designs once,
where the reader first encounters them.

## Distribution

The repository retains its proprietary license and disables registry publication.
The [changelog](CHANGELOG.md) lists the candidate's changes.
