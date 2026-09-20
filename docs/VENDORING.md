# Keep Zergraph source in your application

Use Zergraph as a Cargo dependency, keep a local copy of its crate, or copy its four
source modules into your application's crate. A source copy can be maintained by
your team and coding agents. Record the revision you copied and keep the license.

## Choose the boundary

| Approach | Use it when |
|---|---|
| Pinned Cargo dependency | You want to consume upstream releases |
| Local crate under `vendor/zergraph` | You want editable source with its existing tests and module layout |
| Modules inside your application crate | You want to own and adapt the implementation directly |

A local crate still appears in Cargo's dependency graph, but its source lives in your
repository. Copying the modules removes the separate Zergraph crate dependency.
Both approaches still need Serde, serde_json, and UUID unless you change those parts.
Copying source does not by itself make the code faster or remove their build cost.

## Keep a local crate

Copy the reviewed repository into `vendor/zergraph`, excluding `.git` and `target`.
Retain `src`, `tests`, their fixtures, `Cargo.toml`, `Cargo.lock`, and the license files
`LICENSE.md`, `LICENSE-MIT`, and `LICENSE-APACHE`.
Examples, benchmarks, and docs provide useful checks for later edits.

Add this to your application's manifest:

```toml
[dependencies]
zergraph = { path = "vendor/zergraph" }
```

Run its tests after an edit:

```sh
cargo test --manifest-path vendor/zergraph/Cargo.toml --all-targets --locked
cargo test --manifest-path vendor/zergraph/Cargo.toml --doc --locked
```

## Copy modules into your crate

1. Copy `src/lib.rs` to your application's `src/zergraph/mod.rs`.
2. Copy `src/state.rs`, `src/snapshot.rs`, and `src/sync.rs` into `src/zergraph`.
3. Keep `LICENSE.md`, `LICENSE-MIT`, and `LICENSE-APACHE` beside the copied source.
4. In the three child modules, change their leading `use crate::` imports to
   `use super::`.
5. In `mod.rs`, qualify the three local module imports as `self::snapshot`,
   `self::state`, and `self::sync`. Leave external dependency imports unchanged.
6. Add `pub mod zergraph;` to your application's `src/lib.rs`, or `mod zergraph;`
   to a binary's `src/main.rs`.

Add the direct dependencies to your application's manifest:

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = { version = "1", features = ["float_roundtrip"] }
uuid = { version = "1", features = ["v4", "serde"] }
```

Application code can now use `crate::zergraph::{Graph, EdgeKey}`. Update imported
examples and doctests to your application's module path. A library named `my_app`
uses `my_app::zergraph` from integration tests and doctests.

Copy `tests` and its `fixtures` directory. Change each `zergraph::` import to your
application's module path. Retain this test dependency if using the supplied tests:

```toml
[dev-dependencies]
proptest = { version = "1", default-features = false, features = ["std"] }
```

This module-copy layout was checked in a separate consumer crate with all 43
integration tests and the doctest. Its manifest contains no Zergraph dependency.

## Keep policy in the host

The core owns register stamps, merge validation, deletion behavior, graph views,
and snapshot and delta formats. These rules make independent replicas agree.

The host owns peer selection, copy budgets, pending sends, retries, storage,
transport, compression, scheduling, and what to do when a limit is reached. The
[bounded-sync example](../examples/bounded_sync.rs) supplies ordinary host code for
copy limits. `checkpoint_with_limit` is a small core helper because it must count
hidden records and deletion markers before copying them. Ordinary visible graph
views cannot provide that count.

Prefer a few functions in the host to a generic policy framework in the library.
Do not add a configuration switch for every possible fork. Keep the default core
small, and make a source change when one application needs a different tradeoff.

## Maintain a source copy

Record the source revision and describe local changes next to the vendored code.
An agent can compare later upstream changes, apply selected fixes, and run the
retained tests and workload benchmarks. Decide which updates to accept rather than
replacing a modified copy blindly.

For a host that rarely reads incoming edges, removing the incoming-edge index trades
query speed for memory and simpler restore. A fixed-schema host might replace JSON
values. A host with its own wire format might replace the codec. These are source
fork options, not runtime settings or tested alternative implementations.

After changing those parts, preserve fresh writer identities, atomic rejection,
merge convergence, deletion and revival, exact transport, and snapshot compatibility
if old data must remain readable. Keep the tests for duplicated and reordered deltas
and checkpoint knowledge gaps when changing synchronization.
