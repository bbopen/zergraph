# Application ideas that share one small graph

These are hypotheses and example patterns, not claims of novelty or validated demand. Model data extraction, planning, ranking, geometry, documents, and transport in the host application. The graph stores relationships and their properties.

| Application | Model | Why merge helps |
|---|---|---|
| Coding-agent evidence | Claims, code revisions, commands, outputs; `supports`, `contradicts`, `ran-against` | Independent reviewers contribute evidence without sharing one live session. |
| Worktree handoff | Changes, tests, artifacts, decisions; `depends-on`, `validated-by`, `supersedes` | A handoff can be reconstructed after disconnected work. Git continues to own source history. |
| Field inspection | Assets, observations, photos, follow-up tasks; `observed-at`, `evidence-for` | Technicians and robots can record relationships before reconnecting. |
| Dataset lineage | Data slices, scripts, parameters, models, evaluations; `derived-from`, `evaluated-by` | Provenance travels between secure workstations and training machines. |
| Incident investigation | Services, symptoms, hypotheses, experiments; `depends-on`, `rules-out` | Partitioned investigators retain independently collected evidence. |
| Specimen tracking | Samples, locations, recordings, determinations, lab results | Field and lab observations can be joined after offline collection. |
| Archaeological fragments | Fragments, excavation contexts, scans, proposed joins | AI-generated match suggestions and curator corrections remain separately inspectable. |
| Repair and reuse | Salvaged parts, equipment, dimensions, test evidence; `candidate-for`, `tested-with` | Separate workshops can pool compatibility observations without assuming every suggestion is true. |
| Scientific anomaly comparison | Anomalies, instruments, calibration versions, environmental events | Agents can suggest links across experiments and retain competing explanations with evidence. |
| Cross-domain failure analogies | Software failure, physical fault, experiment, proposed mechanism | AI can propose that two failures share a mechanism; the graph preserves the analogy, counterexamples, and tests as reviewable assertions. |

## Three useful application patterns

**Evidence, rather than a verdict.** Give each claim/observation a separate node ID. Attach its source/model/version as properties, then link supporting and contradicting evidence. Two independent assertions are distinct records, so both survive. Writing both assertions into the same `status` property would intentionally invoke LWW and discard one from the visible value.

**Correction with retained identity.** A technician removes an incorrect relationship and adds the corrected one. Another adds unrelated photo evidence. Complete snapshots merge both changes. Removing a node hides its links; using the same ID later revives it. A replacement physical item should get a fresh identity.

**Portable lineage.** Store artifact IDs, hashes, or URIs as properties and edges; keep payloads outside the graph. Exchange snapshots through any caller-provided medium, restore with a fresh writer, and inspect the same deterministic relationships.

AI may make candidate extraction and cross-domain matching economical. Its output still needs explicit identity and provenance; convergence cannot turn an unsupported suggestion into a fact. These workflows are worth trying because the first experiment can be a small fixture and a relationship question rather than a platform.

## Where the library is a poor fit

Single-writer data often needs only a map, `petgraph`, or SQLite. Collaborative prose may fit a document CRDT better. Task exclusivity, robot reservations, safety control, automatic truth selection, and high-rate telemetry have requirements this graph does not provide. Large/high-churn retained graphs may need a different synchronization or storage design.

Before adding a feature, demonstrate an application whose relationship question cannot be answered through the existing node/edge/property/snapshot surface, and compare the simpler alternative. A useful experiment measures saved reconciliation work, retained state size, and time to answer the question.
