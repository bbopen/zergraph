# Cookbook

These five examples use the public API. Each merges independent changes, encodes a
snapshot, restores the graph, and checks the result. Run the commands from the
repository root.

For new application designs, see [Application ideas](APPLICATIONS.md).
For storage and transport, see [Integrate Zergraph](INTEGRATION.md).

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
