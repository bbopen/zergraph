# Zergraph cookbook

These local-first recipes use the synchronous property graph: caller-owned node IDs,
directed labelled `EdgeKey`s, owned JSON properties, independent UUID writers, and
complete snapshots. Applications choose how bytes travel and persist; the library
opens no connection and starts no background work.

## Three rules

1. A complete snapshot includes hidden nodes, edges, and property deletions. Exchange
   it whole, merge it repeatedly in any order, and restore with
   `Graph::from_snapshot(Snapshot::from_bytes(... )?)` when needed.
2. Membership and each property key are LWW registers ordered by logical counter and
   writer UUID. JSON `null` is a value; removing a property is a different retained
   write. The result is deterministic, not consensus or factual reconciliation.
3. An edge key is exactly `(source, label, target)`. For disagreement, make separate
   **assertion nodes** with distinct IDs and properties, each linked to the same
   subject. Do not put competing claims in one `status` property or duplicate one
   edge triple.

## 1. Coding-agent evidence

**Question:** What test outputs and reviews support this change, including a retracted
stale result?

**Tiny graph:** `change`, `run`, and `claim` nodes; `supports`, `validated-by`, and
`reviews` edges; output hashes or revision IDs as properties.

**Concrete adaptation:** Start with runnable [evidence.rs](../examples/evidence.rs).
Fork two reviewers, let one replace old test evidence and the other add a review, then
merge their snapshots. For competing agent conclusions, use `claim:agent-a` and
`claim:agent-b`, each linked `about` the change.

**Boundary:** Git keeps source history and a person or existing review system decides
the merge. Zergraph does not run tests, extract claims, or assign exclusive work.

## 2. Worktree handoff

**Question:** Which branch, test, artifact, and review still describe a handoff after a
laptop reconnects?

**Tiny graph:** `change:42`, `branch:fix-42`, `test:17`, `review:alice`, and
`artifact:log-hash`; `contains`, `validated-by`, `reviewed-by`, and `attaches` edges.

**Concrete adaptation:** Make one fork per disconnected worker. Each adds its own
`review:*` or `assertion:*` node instead of competing on `change:42.review_status`.
Merge snapshot files into either graph and use `outgoing("change:42")` and
`incoming("change:42")` for a stable summary.

**Boundary:** This records relationships. It is not a task queue, lock, branch merge
mechanism, or replacement for connected issue tracking.

## 3. Robot and technician inspection packet

**Question:** Which independent observations still describe this asset after a vehicle
gateway receives them?

**Tiny graph:** `asset`, `observation:<writer>:<asset>`, `photo`, and `note` nodes;
`observes` and `evidence-for` edges. Put a temperature, model version, or evidence hash
on each observation node.

**Concrete adaptation:** Run [swarm.rs](../examples/swarm.rs) for independent writers,
delayed/deduplicated delivery, and snapshot restoration. Use
[field_inspection.rs](../examples/field_inspection.rs) when one current property needs
correction, a note needs removal, or `null` must differ from an absent work order.

**Boundary:** The graph stores observations, never commands a robot or clears a safety
condition. Telemetry and imagery remain external payloads.

## 4. Dataset-to-evaluation lineage

**Question:** Which data set and script were associated with this evaluation after a
secure workstation and training box worked apart?

**Tiny graph:** `dataset`, `script`, `model`, and `evaluation` nodes; `trained-on`,
`uses`, and `evaluates` edges; hashes, parameters, and metrics as properties.

**Concrete adaptation:** Run [lineage.rs](../examples/lineage.rs). Its two forks add
independent training and evaluation facts, then a complete snapshot is merged and
restored before the lineage question is checked.

**Boundary:** This is an inspectable relationship map, not an artifact store, experiment
runner, evaluator, or substitute for MLflow/LIMS when those are available.

## 5. Incident hypotheses

**Question:** Which observations rule out or support an investigation path while the
incident room is partitioned?

**Tiny graph:** `service`, `symptom`, `hypothesis`, `experiment`, and `assertion` nodes;
`depends-on`, `observed`, `supports`, and `rules-out` edges.

**Concrete adaptation:** Give each observation an ID such as `assertion:host-b:trace-17`
and attach an immutable log hash. A reviewer can add
`decision:incident-9 --assesses--> assertion:host-b:trace-17` without removing another
investigator's finding. Exchange complete snapshots at each handoff.

**Boundary:** A graph edge does not establish root cause or authorize remediation. Use
the incident process for both.

## 6. Repair and reuse evidence

**Question:** Can this salvaged part fit a machine, and which two shops tested that
proposition?

**Tiny graph:** `part`, `machine`, and `assertion` nodes; `candidate-for` and `tests`
edges; dimensions, test-jig URI, and verdict as properties.

**Concrete adaptation:** Run [repair.rs](../examples/repair.rs). It preserves a `fits`
and a `does-not-fit` verdict as separate assertion nodes after independent merge and
restore. A later decision can link to either assertion.

**Boundary:** It does not perform engineering validation or certify a repair. Use a fresh
ID for a replacement part; reviving an ID means the same entity returns with retained
properties and live edges.

## 7. Specimen and archaeology provenance

**Question:** Which context, scan, sample, and interpretation connect to this specimen
when field and lab records meet later?

**Tiny graph:** `locus`, `fragment`, `sample`, `scan`, and `assertion` nodes;
`from-context`, `depicts`, `sampled-from`, and `interprets` edges.

**Concrete adaptation:** A field team adds `scan:sha256:... --depicts--> fragment:41`; a
lab independently adds `sample:lab-8 --sampled-from--> fragment:41`. Store a vision
model's candidate join as `assertion:vision-1`, with model and evidence properties,
linked `interprets` to the fragment. A conservator's different assertion gets its own ID.

**Boundary:** GIS geometry, photogrammetry, catalog media, and archaeological judgment
remain in specialist systems. This is speculative: no AI output becomes fact by merging.

## 8. Cross-domain anomaly comparison

**Question:** Did a software failure, lab anomaly, and physical fault have a plausibly
similar mechanism worth testing?

**Tiny graph:** `incident`, `experiment`, `fault`, `mechanism`, `assertion`, and `test`
nodes; `exhibits`, `suggests`, `contradicts`, and `tested-by` edges.

**Concrete adaptation:** A model creates `assertion:model-42` with its prompt, version,
source hashes, and a candidate `suggests` edge. An engineer or scientist adds separately
named counterexamples. On merge, inspect both assertions and evidence before creating a
test node.

**Boundary:** This surprising combination is deliberately speculative. Zergraph performs
no semantic search, inference, ranking, causal analysis, or model execution; it retains
reviewable host-supplied links.

## 9. Air-gapped release exceptions

**Question:** Which component, finding, compensating control, and reviewer assertion form
the evidence set for a release exception?

**Tiny graph:** `component`, `finding`, `control`, `decision`, and `assertion` nodes;
`affects`, `mitigates`, `waives-for`, and `asserts` edges.

**Concrete adaptation:** Use package URLs, scanner-result hashes, and ticket IDs as JSON
properties. Give each review a separate assertion node. A complete snapshot can travel
with the release bundle and restore exactly for later inspection.

**Boundary:** The graph is not a CVE feed, signer, policy engine, or system of record.
Keep authorization and security decisions outside it.

## Choosing a smaller tool

Use a map, SQLite, or `petgraph` when one local writer owns the data. Use a document CRDT
for collaborative prose. This library earns its place only when independent writers need
labelled relationships and selected properties to converge after disconnected work. It
does not provide resource reservation, task claims, transport authentication, history
compaction, or truth selection.
