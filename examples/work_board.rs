//! A local work-board fixture; application acknowledgements and transport are simulated.
use zergraph::{Checkpoint, Delta, EdgeKey, Graph, Snapshot, Value};

fn record_attempt(
    graph: &mut Graph,
    attempt: &str,
    evidence: &str,
    result: &str,
) -> Result<(), zergraph::Error> {
    graph.add_node(attempt)?;
    graph.add_node(evidence)?;
    graph.set_node_property(attempt, "result", result)?;
    graph.set_node_property(evidence, "kind", "bench record")?;
    graph.add_edge(EdgeKey::new(attempt, "tests", "candidate:filter-17"))?;
    graph.add_edge(EdgeKey::new(evidence, "supports", attempt))?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut publisher = Graph::new();
    publisher.add_node("candidate:filter-17")?;
    publisher.set_node_property("candidate:filter-17", "revision", "r3")?;

    // A new peer has no common state. The default checkpoint has no retained stamps.
    let mut acknowledged = Checkpoint::default();
    assert!(!publisher.delta_since(&acknowledged).is_empty());
    let bootstrap_candidate = publisher.checkpoint();
    let bootstrap_bytes = publisher.snapshot().to_bytes()?;
    let mut board = Graph::from_snapshot(Snapshot::from_bytes(&bootstrap_bytes)?);

    // The application receives an acknowledgement for this exact bootstrap.
    acknowledged = bootstrap_candidate;

    // Independent writers use distinct attempt and evidence IDs, so both results survive.
    let mut lab_a = publisher.fork();
    let mut lab_b = publisher.fork();
    record_attempt(
        &mut lab_a,
        "attempt:lab-a:trial-1",
        "evidence:lab-a:trial-1",
        "passes",
    )?;
    record_attempt(
        &mut lab_b,
        "attempt:lab-b:trial-1",
        "evidence:lab-b:trial-1",
        "fails",
    )?;
    publisher.merge_with_changes(&lab_a.snapshot())?;
    publisher.merge_with_changes(&lab_b.snapshot())?;

    // One in-flight batch: capture its candidate before deriving the delta, retain
    // the old acknowledgement until the receiver applies it and the ACK arrives.
    let candidate = publisher.checkpoint();
    let pending_bytes = publisher.delta_since(&acknowledged).to_bytes()?;

    // Simulate a dropped send. No state or checkpoint changes at the receiver.
    assert!(board.node("attempt:lab-a:trial-1").is_none());

    // Retry exactly the same batch while still using the old acknowledged checkpoint.
    let received = Delta::from_bytes(&pending_bytes)?;
    let changes = board.apply_delta_with_changes(&received)?;
    assert_eq!(
        changes.nodes,
        vec![
            "attempt:lab-a:trial-1".to_string(),
            "attempt:lab-b:trial-1".to_string(),
            "evidence:lab-a:trial-1".to_string(),
            "evidence:lab-b:trial-1".to_string(),
        ]
    );
    assert_eq!(
        changes.edges,
        vec![
            EdgeKey::new("attempt:lab-a:trial-1", "tests", "candidate:filter-17"),
            EdgeKey::new("attempt:lab-b:trial-1", "tests", "candidate:filter-17"),
            EdgeKey::new(
                "evidence:lab-a:trial-1",
                "supports",
                "attempt:lab-a:trial-1",
            ),
            EdgeKey::new(
                "evidence:lab-b:trial-1",
                "supports",
                "attempt:lab-b:trial-1",
            ),
        ]
    );

    // Only this simulated application ACK promotes the peer's checkpoint.
    acknowledged = candidate;
    assert!(publisher.delta_since(&acknowledged).is_empty());

    // Duplicate delivery changes no stored record and reports no view refresh work.
    let duplicate = board.apply_delta_with_changes(&Delta::from_bytes(&pending_bytes)?)?;
    assert!(duplicate.is_empty());

    assert_eq!(
        board
            .node("attempt:lab-a:trial-1")
            .unwrap()
            .property("result"),
        Some(&Value::from("passes"))
    );
    assert_eq!(
        board
            .node("attempt:lab-b:trial-1")
            .unwrap()
            .property("result"),
        Some(&Value::from("fails"))
    );
    assert_eq!(board.edges().count(), 4);

    // A restored or replacement peer is bootstrapped again; do not reuse
    // `acknowledged` as proof that it has this peer's prior state.
    let restored = Graph::from_snapshot(Snapshot::from_bytes(&board.snapshot().to_bytes()?)?);
    assert_eq!(restored.snapshot(), board.snapshot());
    println!("Independent work-board results converged; duplicate delta was a no-op.");
    Ok(())
}
