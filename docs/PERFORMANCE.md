# Performance

These measurements compare two Zergraph revisions on an Apple M3 with 8 CPU cores
and 24 GiB RAM. Both use native macOS ARM64, rustc 1.97.1, and Cargo's default release
optimization. They do not compare Zergraph with other libraries.

- Baseline: `de2029f85763f870eb4ead11463effdb698f092a`.
- Optimized core: `6d73ba9830c45e7df70a0ae928f274227e7d6b18`.

## Query and snapshot timings

The fixture has 1,024 nodes and 4,096 directed edges, with four outgoing edges per
node. Each node has two properties; each edge has one. The table reports medians
of three process medians, with seven samples per operation in each process.
Baseline and optimized runs alternated. Timing batches ran sequentially with the
same fixture and benchmark program.

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

A ratio below 1 means the optimized version was slower. The incoming index makes
restore take about 0.71 ms, up from 0.021 ms. Fork takes about 1.06 ms, up from
0.73 ms. Snapshot creation and decoding are nearly unchanged.

Adding the separately measured decode and restore medians gives 4.85 ms for the
optimized version and 4.18 ms for the baseline. This sum estimates the combined
cost; the combined operation was not timed directly.

## Implementation changes

Outgoing queries seek to an ordered range of edges with the requested source.
Incoming queries use an index from target IDs to edge keys. Both apply the same
visibility rule as edge lookup.

Merge validates all incoming registers before changing state. It then applies a
plan containing only changed records. When keys align, ordered iteration avoids
repeated lookups. Other key layouts use map lookups.

Setters and decoding normalize JSON object order. Encoding borrows stored values
without cloning the graph. The version-1 format remains compatible with the
original packaged snapshot fixture.

The core grew from 595 to 667 source lines, including documentation, across three
modules. It has three direct runtime dependencies. Check the current counts with:

```sh
wc -l src/*.rs
cargo tree --locked --edges normal --depth 1
```

## Larger graph and memory

A separate fixture used 10,000 nodes and 40,000 edges. It ran seven samples, with
100 repetitions for neighbor queries and one repetition for each expensive operation.
Outgoing queries took about 0.43 µs and incoming queries took about 0.76 µs. A full
merge with one changed property took about 5.01 ms. Encoding took about 17.44 ms.

That fixture ran in one process per variant, so it has less noise control than the
primary table. The measurements do not cover dense graphs, high-degree nodes, heavy
deletion, long IDs, or large JSON values.

A separate memory probe constructed the same larger graph and input-ID list without
snapshots or timing batches. Peak resident set size, or RSS, rose from 61.0 MiB to
72.8 MiB, about 19%. RSS includes process and allocator overhead. It is not an exact
measurement of live graph memory.

The primary fixture's snapshot remains 1,619,049 bytes. Synchronization still sends
complete state. These optimizations do not reduce snapshot bandwidth or reclaim
records after deletion.

## Build times

Library builds into fresh target directories took 3.40 s for the baseline and
4.13 s for the optimized version. Downloaded dependencies were already cached.
Both no-change builds took 0.03 s. Compiler CPU time was about 6.6 s in both fresh
builds. These single runs do not establish a build-speed change.

Removing unused proptest fork, timeout, and bit-set features reduced the lockfile
from 61 to 48 package records. That count includes the crate and all locked targets;
it is not a runtime dependency count. Property generation, shrinking, and regression
persistence remain enabled. Feature branches run one PR CI matrix instead of
duplicate push and PR matrices.

## Reproduce the measurements

Run the benchmark with timing enabled:

```sh
cargo bench --locked --bench perf -- --measure
```

To repeat measurements without another Cargo cycle, build the executable first:

```sh
cargo bench --bench perf --no-run --locked
```

Run the executable path that Cargo prints with `--measure`. The benchmark accepts
these environment variables:

| Variable | Example value |
|---|---|
| `ZERGRAPH_BENCH_SMALL_N` | `64` |
| `ZERGRAPH_BENCH_MEDIUM_N` | `1024` |
| `ZERGRAPH_BENCH_DEGREE` | `4` |
| `ZERGRAPH_BENCH_SAMPLES` | `7` |
| `ZERGRAPH_BENCH_ONLY` | `outgoing,incoming` |

For the macOS memory probe, replace `path/to/perf` with that executable path:

```sh
ZERGRAPH_BENCH_MEDIUM_N=10000 /usr/bin/time -l path/to/perf --memory
```

`--memory` constructs the graph and prints its node and edge counts. The size override
selects the recorded 10,000-node fixture; the default is 1,024 nodes. The macOS
`time` command reports maximum resident set size in bytes.

The program emits raw samples, medians, and snapshot sizes as TSV.
[Comparison data](benchmarks/comparison.tsv) contains the recorded results.
Ordinary tests run a small benchmark smoke check; timing requires `--measure`.

Snapshot, fork, encoding, and decoding timings include destruction of the returned
allocation. Restore and changed-merge timings exclude setup and destruction. Query
inputs use `black_box`. Ring-shaped fixtures keep the degree fixed for reproducibility.

## Parallel execution

The [swarm example](../examples/swarm.rs) uses four standard scoped threads. Each
thread creates independent observations. The example checks convergence after
delayed, duplicated, and reordered snapshot delivery, then checks the restored state.
It demonstrates parallel writers and merge behavior. It measures no parallel speedup
and runs no robot hardware.

The [contributor guide](../CONTRIBUTING.md) lists the release checks. Tests cover merge
histories, adjacency, hidden edges, failed-merge atomicity, nested JSON order, exact
number transport, and snapshot compatibility.

## Optimization targets

The optimization experiment used these local targets: both neighbor queries below
2 µs, changed merge below 0.5 ms, encoding below 1.6 ms, passing tests, and fewer than
750 core lines. The final measured variant met those targets.

The merge-plan experiment and reporting followed the
[autoresearch skill](https://github.com/wjgoarxiv/autoresearch-skill/blob/c4c5948994dd9000eb19d69afcb8d62c05bf0112/skills/autoresearch/SKILL.md).
The baseline and initial index experiments preceded that step. These targets apply
to the recorded fixture and hardware.
