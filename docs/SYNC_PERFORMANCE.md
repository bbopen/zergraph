# Incremental-sync performance

These measurements use an Apple M3 with 8 CPU cores, 24 GiB RAM, native macOS
ARM64, rustc 1.97.1, and Cargo's default release profile. They compare implementations
of the same new API. They do not compare other libraries or measure a network.

- Initial sync implementation: `fb912f90bc3da96e0dfeb785dcb8fe541ffc410f`.
- Ordered-key comparison: `c50c4737fcf4c8c9a383cf9f0bbbec759f368bd0`.
- Compact checkpoints: `22f5458372851eb9c22fa3e8246c8f6fbb6d8714`.

## Fixture and timings

The graph has 1,024 nodes and 4,096 edges in a ring, with four outgoing edges per
node. Each node has a 128-byte string and an integer property. Each edge has one
integer property. One existing node property changes before exchange. This fixture
differs from the earlier [snapshot benchmark](PERFORMANCE.md).

Each value below is the median of three process medians, with seven samples and
100 repetitions per sample. Variants ran sequentially. Each call has its own timer;
receiver setup and destruction of returned values and receivers are excluded.
Timer overhead remains included and matters for sub-microsecond calls.

| Operation | Initial, µs | Ordered keys, µs | Compact checkpoints, µs |
|---|---:|---:|---:|
| `checkpoint` | 711.144 | 682.524 | 492.750 |
| `delta_unchanged` | 631.186 | 170.379 | 177.314 |
| `delta_one_property` | 629.811 | 169.796 | 176.420 |
| `delta_encode` | 0.363 | 0.361 | 0.364 |
| `delta_decode` | 0.585 | 0.578 | 0.582 |
| `apply_delta_changed` | 0.348 | 0.348 | 0.342 |
| `apply_delta_duplicate` | 0.165 | 0.168 | 0.155 |
| `apply_delta_reported` | 0.471 | 0.489 | 0.493 |
| `full_merge_changed` | 309.230 | 299.659 | 301.605 |
| `full_merge_reported` | 306.423 | 305.610 | 303.888 |
| `full_snapshot_encode` | 1386.571 | 1341.146 | 1327.846 |

Ordered iteration avoids repeated tree lookups when the graph and checkpoint have
matching entity keys. Differing keys still use exact lookup. Compact checkpoints
store each entity's property stamps in a sorted array and use binary search.

The final geometric mean of unchanged and one-property delta generation is 72%
lower than the initial version. Compact checkpoints make this scan about 4% slower
than the ordered-map variant, while making checkpoint capture about 28% faster.
The memory reduction below is why the compact variant was kept.

An empty delta encodes to 50 bytes. The one-property delta is 294 bytes; the full
snapshot is 1,746,030 bytes. These sizes depend on IDs, properties, and which records
changed. Edge updates also include endpoint membership records.

These application timings exclude transport, acknowledgments, storage, and view
refresh. A complete outgoing batch also pays for checkpoint capture, delta
generation, and encoding. A tiny receive time does not mean the whole sync costs
that amount. Delta generation still scans retained state.

## Checkpoint memory

A separate memory fixture has 10,000 nodes and 40,000 edges with the same property
shape. Each fresh process holds one graph and the stated number of checkpoints.
Numbers are median peak RSS from three processes per case, measured with macOS
`/usr/bin/time -l`. They include allocator and process overhead, not just live objects.

| Checkpoints | Ordered maps, MiB | Compact arrays, MiB |
|---|---:|---:|
| 0 | 73.30 | 73.31 |
| 1 | 119.47 | 90.89 |
| 16 | 732.08 | 273.61 |

For 16 peers, additional RSS above the graph-only process falls by about 70%.
One extra confirmation measurement reproduced the reduction. Checkpoints remain
proportional to retained entity IDs, property names, and stamps. They are payload-free,
but not free of memory cost. Bound the number of peers and the size of each graph.

## Research and code size

The first autoresearch loop targeted a 35% reduction in delta generation. It stopped
after one candidate exceeded that target; an extra timing run confirmed the result.
The memory measurements then justified a separate loop targeting a 50% checkpoint
memory reduction, with a 10% maximum regression in delta and checkpoint generation.
One candidate met those conditions, followed by a memory confirmation.

Both loops used the
[autoresearch skill](https://github.com/wjgoarxiv/autoresearch-skill/blob/c4c5948994dd9000eb19d69afcb8d62c05bf0112/skills/autoresearch/SKILL.md),
mechanical evaluators, tests and Clippy as guards, and a five-minute subprocess limit.
The record describes two bounded experiments, not a proof of optimal performance.

The measured core has 868 lines in four Rust modules, including comments and API
docs, up from 667 lines before sync support. The later optional checkpoint-cap API
brings the release to 884 lines. Later cap timing runs overlapped a CPU-heavy
workload and are excluded from the comparison; they do not establish cap overhead. It still has three direct runtime dependencies.
No operation history, background service, transport, or thread pool was added.

## Reproduce

```sh
cargo bench --locked --bench sync -- --measure
```

The timing controls are `N=1024`, `SAMPLES=7`, and `ITERATIONS=100`. Degree is fixed
at four. `ONLY=checkpoint,checkpoint_capped,checkpoint_rejected` selects the checkpoint
metrics. A capped capture scans entity counts before copying; rejection copies nothing. For memory measurements, build once and use the executable path Cargo prints:

```sh
cargo bench --locked --bench sync --no-run
N=10000 CHECKPOINTS=0 /usr/bin/time -l path/to/sync --memory
N=10000 CHECKPOINTS=1 /usr/bin/time -l path/to/sync --memory
N=10000 CHECKPOINTS=16 /usr/bin/time -l path/to/sync --memory
```

The benchmark emits TSV samples and medians. Without `--measure` or `--memory`, it
only checks a small fixture. [Comparison data](benchmarks/sync-comparison.tsv) records
the timing medians, and [memory data](benchmarks/sync-memory.tsv) records the RSS runs.
