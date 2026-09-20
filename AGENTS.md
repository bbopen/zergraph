# Working on Zergraph

Read README.md, docs/SEMANTICS.md, and CONTRIBUTING.md before editing.

- Keep the core synchronous. The application owns storage and transport.
- Keep policy in host code. Add a core primitive when the host cannot implement it
  cheaply through the public API. Preserve the option to vendor the source as modules.
- Preserve fresh writer identities, atomic merge rejection, deterministic views,
  deletion and revival rules, and complete snapshots.
- Use the public API in examples. Keep domain code out of the core.
- Run focused checks during edits and the documented release checks for behavior
  changes. Execute example main functions as well as compiling them.
- Enable benchmark timings only with `--measure`.
- Compare performance on the same workload. Report memory, restore, and fork costs.
- Label untested hardware ports and cluster designs as proposals.
- Write plain technical English. Cut hype, repeated caveats, and invented jargon.
  Keep runnable task guides separate from reference and unbuilt application ideas.
- Preserve the license, publication setting, and snapshot compatibility unless the
  user requests a change to them.
