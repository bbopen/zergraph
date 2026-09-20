# Working on Zergraph

Read README.md, docs/SEMANTICS.md, and CONTRIBUTING.md first. The core is the three Rust modules under src/. Cookbooks, research notes, benchmarks, tests, and examples explain and validate the core; they are not additional runtime subsystems.

- Keep the crate synchronous and preserve its caller-owned storage/transport boundary.
- Preserve distinct writer identity, atomic merge rejection, deterministic views, deletion/revival behavior, and complete snapshot state.
- Use the public API in examples. Put domain-specific modeling in examples and docs/COOKBOOK.md.
- Run a focused check during edits. Run the documented release checks before finalizing a behavior change; execute example main functions, not only cargo test --examples.
- Do not run timing campaigns through normal tests. The benchmark binary requires --measure.
- Measure performance changes on the same workload and report memory, restore, and fork tradeoffs. Keep benchmark claims scoped to their recorded hardware and graph shape.
- Describe hardware targets and cluster compositions as reasoned designs until they have actual execution evidence. The library currently exchanges whole snapshots and does not implement distributed sharding or global queries.
- Update documentation and snapshot compatibility tests when the contract changes. Preserve the existing license and publication setting unless the user requests a distribution change.

The current user task's explicit instructions take precedence over this guidance. Routine reversible implementation, documentation, and validation work does not require another approval step.
