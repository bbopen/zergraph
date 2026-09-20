//! All records are fixtures; this example performs no model or network calls.
use zergraph::{EdgeKey, Graph, Snapshot, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut graph = Graph::new();
    graph.add_node("claim:migration-compatible")?;
    graph.add_node("run:old-tests")?;
    graph.set_node_property(
        "claim:migration-compatible",
        "text",
        "Migration preserves existing records",
    )?;
    let old_support = EdgeKey::new("run:old-tests", "supports", "claim:migration-compatible");
    graph.add_edge(old_support.clone())?;

    // Two local reviewers start from the same graph with distinct writer IDs.
    let mut researcher = graph.fork();
    let mut reviewer = graph.fork();
    researcher.add_node("run:current-tests")?;
    researcher.set_node_property("run:current-tests", "revision", "candidate")?;
    let current_support = EdgeKey::new(
        "run:current-tests",
        "supports",
        "claim:migration-compatible",
    );
    researcher.add_edge(current_support.clone())?;
    researcher.set_edge_property(&current_support, "passed_cases", 12)?;

    reviewer.set_node_property("claim:migration-compatible", "reviewed", true)?;
    reviewer.remove_edge(&old_support)?;

    graph.merge(&researcher.snapshot())?;
    graph.merge(&reviewer.snapshot())?;
    assert!(graph.edge(&old_support).is_none());
    assert_eq!(
        graph
            .edge(&current_support)
            .unwrap()
            .property("passed_cases"),
        Some(&Value::from(12))
    );
    assert_eq!(
        graph
            .node("claim:migration-compatible")
            .unwrap()
            .property("reviewed"),
        Some(&Value::from(true))
    );

    // Bytes can be saved by the caller. Restoring assigns a fresh writer ID.
    let bytes = graph.snapshot().to_bytes()?;
    let mut restored = Graph::from_snapshot(Snapshot::from_bytes(&bytes)?);
    assert!(restored.edge(&old_support).is_none());
    assert!(restored.edge(&current_support).is_some());

    // A stale replica must not resurrect the explicitly retracted relationship.
    restored.merge(&researcher.snapshot())?;
    assert!(restored.edge(&old_support).is_none());
    println!("Evidence reconciled: current test run supports the reviewed claim.");
    println!("Old support remains retracted after snapshot restore and stale merge.");
    Ok(())
}
