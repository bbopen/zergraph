# Measured performance and tradeoffs

Measured on an Apple M3 (8 CPU cores, 24 GiB RAM), native macOS ARM64, rustc 1.97.1, default Cargo release optimization. Baseline is `de2029f85763f870eb4ead11463effdb698f092a`. These are local synthetic measurements, not robot deadlines, a cross-platform performance certification, or a comparison against other graph libraries.

## Representative sparse graph

1,024 nodes, 4,096 directed edges (degree four), two properties per node and one per edge. Seven samples per operation, three separate processes per variant, alternating baseline and optimized runs. Values below are medians of process medians; timing batches do not run concurrently. Both variants use the same fixture and harness. The first baseline run preceded the explicit benchmark opt-in flag; its measured workload was identical.

| Operation | Baseline, microseconds | Optimized, microseconds | Baseline / optimized |
|---|---:|---:|---:|
| `node_lookup` | 0.044 | 0.043 | 1.02x |
| `outgoing` | 446.112 | 0.524 | 851.43x |
| `incoming` | 451.120 | 0.749 | 602.67x |
| `snapshot` | 696.205 | 703.194 | 0.99x |
| `restore` | 21.162 | 710.393 | 0.03x |
| `fork` | 727.795 | 1058.032 | 0.69x |
| `merge_unchanged` | 1027.057 | 136.923 | 7.50x |
| `merge_changed_property` | 1053.702 | 182.057 | 5.79x |
| `snapshot_encode` | 1981.295 | 1245.431 | 1.59x |
| `snapshot_decode` | 4156.216 | 4141.150 | 1.00x |

Numbers below 1x represent a regression. The incoming index makes restore about 0.71 ms instead of 0.021 ms and fork about 1.06 ms instead of 0.73 ms. Snapshot creation and JSON decoding are essentially unchanged. Combining the separately measured decode and restore medians gives approximately 4.85 ms versus 4.18 ms; this is an estimate, not a directly timed pipeline.

## What changed

- Outgoing queries seek to an existing ordered edge range. Incoming queries use a derived target-to-edge-key index. Both apply the same endpoint visibility predicate, including hidden/deleted state.
- Merge validates all incoming registers before mutation, builds a borrowed plan of changed records, and applies only those records. Aligned key traversal avoids repeated map lookups; sparse or shifted keys fall back to lookup.
- JSON values are normalized when written or decoded. Snapshot encoding borrows these immutable values rather than cloning all entities and properties. The version-1 format is unchanged and an original packaged snapshot is a byte-compatibility fixture.

Production source grew from **595 to 667 lines**, including documentation, across the same three modules. There are still three direct runtime dependencies. No unsafe code, native database dependency, thread pool, async runtime, or parallelism dependency was added.

## Larger fixture and memory cost

A separate 10,000-node/40,000-edge run used seven samples, 100 repetitions for neighbor queries and one repetition for each expensive operation per sample. Outgoing/incoming queries were about 0.43/0.76 microseconds, one-property full merge about 5.01 ms, and encoding about 17.44 ms. This is one process per variant and has weaker noise control than the primary table. Dense graphs, high-degree hubs, heavy tombstone churn, long IDs, and large JSON payloads were not performance-tested.

A separate-process memory probe creates one such graph and the same input-ID list, without snapshots or timing batches. Peak process RSS was **61.0 MiB baseline versus 72.8 MiB optimized**, approximately **19% higher**. RSS includes process overhead and allocator behavior; this is not an exact live-heap measurement. The index intentionally trades memory and construction/restore work for repeated query speed.

Full snapshot sizes are unchanged: **1,619,049 bytes** for the primary fixture. Synchronization still sends complete state; none of these changes reduces network bandwidth or reclaims tombstones.

## Build and development cost

Library-only builds into fresh target directories, with downloaded dependencies already cached, measured **3.40 s baseline and 4.13 s optimized**. Both no-change builds took **0.03 s**. Compiler CPU time was approximately 6.6 s in both cold runs; wall-clock differences include scheduling noise. No build-speed improvement is claimed from these single runs.

Unused proptest fork/timeout/bit-set features were removed while retaining standard property generation, shrinking, and regression persistence. The lockfile decreased from 61 to 48 package records (including zergraph and all locked targets); this is not a count of shipped runtime dependencies. Feature-branch changes now trigger one PR CI matrix instead of duplicate push and PR matrices.

Use `cargo check --lib --locked` during edits, keep Cargo's target directory, and run the focused integration test for the changed behavior. Run the complete checks before committing. Timing loops require explicit `--measure`, so normal tests perform only a tiny benchmark smoke check.

## Reproduce

```sh
cargo bench --bench perf -- --measure
cargo bench --bench perf --no-run --locked
# Run the executable path printed above with --measure to avoid another Cargo cycle.
# Environment controls:
# ZERGRAPH_BENCH_SMALL_N=64 ZERGRAPH_BENCH_MEDIUM_N=1024
# ZERGRAPH_BENCH_DEGREE=4 ZERGRAPH_BENCH_SAMPLES=7
# ZERGRAPH_BENCH_ONLY=outgoing,incoming
# Pass --memory instead for the separate-process graph construction/RSS probe.
```

The harness reports raw samples, medians, and snapshot sizes as TSV. Snapshot/fork/encoding/decoding timings include destruction of the returned allocation. Restore and changed-merge setup/destruction are excluded from their timed windows. Query inputs use black_box; the regular ring fixtures have a fixed degree and are deliberately easy to reproduce. See [comparison data](benchmarks/comparison.tsv).

## Parallel work and verification

`cargo run --locked --example swarm` uses four standard scoped threads to create independent robot observations and verifies convergence through delayed, duplicate, and reordered snapshot delivery. Separate observation IDs retain conflicting readings. This demonstrates caller-controlled parallelism and reconciliation, not a parallel speedup or physical robotics acceptance test.

Local validation passes: 31 integration tests, one doctest, three runnable examples, Clippy with warnings denied, and both normal/preserve_order configurations. Tests cover generated merge histories, index/projection equality, hidden-edge lifecycle, atomic merge rejection, canonical nested JSON, floating-point transport, and compatibility with the previous wire format. No public API was removed by this optimization.

## Experiment method

The first baseline/range/index experiments preceded discovery of the requested skill. The remaining merge-plan experiment and reporting followed the [autoresearch skill](https://github.com/wjgoarxiv/autoresearch-skill/blob/c4c5948994dd9000eb19d69afcb8d62c05bf0112/skills/autoresearch/SKILL.md), read from a local checkout without global installation. Its evaluator met the declared local targets: both queries below 2 microseconds, changed merge below 0.5 ms, encoding below 1.6 ms, preserved tests, and fewer than 750 core lines. These thresholds are experiment choices, not universal performance promises.
