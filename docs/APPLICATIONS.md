# Application ideas

The [cookbook](COOKBOOK.md) contains six runnable examples. The designs below extend
those patterns to other work. They have not been tested as complete applications.
Zergraph stores the relationships; an application captures evidence and uses the result.

## Repair knowledge that travels with equipment

The repair example preserves two workshops' conflicting verdicts. A useful next step
is to attach each verdict to a machine revision, test conditions, and evidence.
A later repairer could find out which substitute parts worked under matching conditions.

Each test needs a separate assertion node. Photos, measurements, and logs can stay
in external storage, with their hashes or IDs in the graph. Workshops can exchange
complete snapshots after offline work. An agent could propose records from notes or
photos, then help find prior tests.

Repair records and compatibility graphs already exist. The
[Open Repair Data Standard](https://standard.openrepair.org/standard.html) describes
repair attempts and outcomes. [Eccenca's product-data case](https://eccenca.com/success-story/product-data-management-system)
describes a graph for finding compatible replacement parts. The opportunity to test
here is sharing conditional pass and fail evidence between independent workshops.

A small pilot can measure time to find an applicable prior test and the number of
failed substitutions repeated. The current example proves that records survive merge;
it does not measure either workflow result.

## Worktree handoff

A handoff can connect a code change to its branch, test runs, artifacts, and reviews.
Each worker adds its own review nodes while Git keeps the source history. On reconnect,
the graph can show which evidence belongs to which revision.

Start with the [evidence example](../examples/evidence.rs). Add branch and artifact
nodes, then link them to the change. Store the exact revision and output hash so the
next worker can check whether the evidence still applies.

## Incident investigation

Separate teams can record symptoms, hypotheses, and experiments while disconnected.
Each observation gets its own ID and source. Edges such as `supports`, `rules-out`,
and `depends-on` connect the evidence without overwriting a competing hypothesis.

The first useful test is whether a new investigator can find the evidence for an
open hypothesis after snapshots merge. Root-cause analysis and remediation stay in
the incident process.

## Field and laboratory provenance

Field teams can link fragments, scans, and excavation contexts. A lab can add samples,
measurements, and interpretations later. Each interpretation needs its own assertion
node, whether it came from a person or a model.

The same pattern can connect biodiversity recordings to specimens and lab results.
Use shared specimen IDs and keep media in the catalog that already stores it. Test
whether a curator can trace an interpretation back to its source after offline exchange.

## Comparing failures across domains

An agent could suggest that a software incident and a physical fault share a mechanism.
The graph can link the proposed analogy, its evidence, counterexamples, and a test.
Each suggestion stays a separate assertion until the application records a decision.

Cross-domain discovery has prior art, including an
[Analogy Search Engine](https://arxiv.org/abs/1812.06974). [Nanoarguments](https://nlnet.nl/project/Nanoarguments/)
works on federated claims and evidence. [Lattice Graph](https://latticegraph.com/fit/radical-ai)
advertises failed-experiment records for scientific agents.

A useful experiment must show that the proposed links help choose better diagnostic
tests than keyword or embedding search. Zergraph supplies the record and merge rules.
Retrieval, comparison, and test selection need application code.

## Air-gapped release evidence

A release bundle can carry a graph of components, findings, controls, and reviews.
Use package IDs, scanner-result hashes, and ticket IDs as properties. Give each
review its own assertion node and link it to the finding it evaluates.

A reviewer can restore the snapshot and inspect the same relationships without the
original service. Signing the bundle and authorizing a release remain part of the
release process.
