# Cookbook

These seven examples use the public API. The snapshot examples merge independent
changes, encode complete state, restore the graph, and check the result. The work
board and bounded-sync examples also send deltas after bootstrap. Run commands from
the repository root.

For new application designs, see [Application ideas](APPLICATIONS.md). For storage
and transport, see [Integrate Zergraph](INTEGRATION.md).

## Bound retained synchronization knowledge

Use this pattern when the application must bound the checkpoint copies it keeps for
a synchronization session.

```sh
cargo run --locked --example bounded_sync
```

The example uses local limits of two checkpoint copies and 128 retained registers.
The two copies are the acknowledged baseline and one pending candidate. A dropped
send retries from the baseline; another candidate is refused while both slots are
occupied. The graph refuses an over-register-budget checkpoint before creating a
partial one. Forgetting peer knowledge is safe: a delta from `Checkpoint::default()`
reboots the peer with complete retained state.

These limits are application policy. They do not impose a byte limit on snapshots or
deltas, and Zergraph never truncates a delta. Defer a send when the application has
no budget for its state or transport bytes; use a complete snapshot only when its
own budget permits it.

[Source: bounded_sync.rs](../examples/bounded_sync.rs)

## Sync a work board after bootstrap

Use this pattern when one application owns a graph, peers first receive its complete
state, and later receive acknowledged batches of changes.

```sh
cargo run --locked --example work_board
```

The example bootstraps a board from a full snapshot. Two independent labs add
different attempts and evidence for the same candidate, including competing results.
It keeps the old checkpoint after a simulated dropped send, retries the same delta,
then promotes the checkpoint only after an application acknowledgement. A duplicate
delivery returns an empty change report.

A checkpoint is valid only for the peer that has acknowledged the corresponding
state. If that peer is restored, replaced, or loses its graph state, send a complete
snapshot and establish a new checkpoint. Keep at most one unacknowledged batch for
a peer unless the application defines a fuller acknowledgement protocol.

[Source: work_board.rs](../examples/work_board.rs)

## Keep test evidence current

Use this pattern to connect a code change to its tests and reviews.

```sh
cargo run --locked --example evidence
```

The example replaces stale test evidence while another writer adds a review.
After merge and restore, the current test supports the claim. The old relationship
stays removed even when an old snapshot arrives again.

Use a separate node ID for each test run and review. Store the source revision and
output hash as properties. Link each result to the change it evaluates.

[Source: evidence.rs](../examples/evidence.rs)

## Correct an inspection record

Use this pattern when one writer corrects a reading while another adds follow-up work.

```sh
cargo run --locked --example field_inspection
```

The example corrects a temperature to 21.3 C, removes an old note, and adds an
inspection schedule. It checks that the correction and schedule survive restore.
It also distinguishes a JSON `null` value from a removed property.

Store facts that can change independently under separate property keys. Reuse the
asset ID for the same physical item. Give a replacement item a new ID.

[Source: field_inspection.rs](../examples/field_inspection.rs)

## Combine robot observations

Use this pattern when robots work apart and exchange observations later.

```sh
cargo run --locked --example swarm
```

Four standard Rust threads record separate robot observations. The example
reorders and duplicates snapshot delivery, then checks that all four replicas
converge. The restored snapshot contains the same state.

Give each observation a distinct ID, even when two robots observe the same asset.
Store the sensor, time, and reading on that observation. Keep images and telemetry
in external storage and put their IDs in the graph. The example simulates data
exchange; your application supplies the robot connection and control logic.

[Source: swarm.rs](../examples/swarm.rs)

## Record dataset and model lineage

Use this pattern to connect a model to its training data, script, and evaluation.

```sh
cargo run --locked --example lineage
```

One writer adds the model's training relationships. Another adds an evaluation and
its accuracy. After merge and restore, the example checks the data, script, and
evaluation links.

Use stable artifact IDs. Store a dataset hash, script revision, parameters, or
metric as properties. Store the artifacts themselves outside the graph.

[Source: lineage.rs](../examples/lineage.rs)

## Preserve conflicting repair results

Use this pattern when two workshops test whether a part fits a machine.

```sh
cargo run --locked --example repair
```

One workshop records `fits`; the other records `does-not-fit`. Each result has its
own assertion node. Both verdicts and their links survive merge and restore.

Add the machine revision, test conditions, and evidence ID to each assertion in
your application. Keep each test separate. Writing both verdicts to one shared
property would select one LWW winner and lose the other visible value.

Use a fresh ID for a replacement part. Re-adding a removed ID restores the same
part's retained properties and still-live edges.

[Source: repair.rs](../examples/repair.rs)
