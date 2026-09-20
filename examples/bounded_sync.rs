//! Checkpoint-copy bounds are application policy, not graph scheduling.
use zergraph::{Checkpoint, Graph};

#[derive(Clone, Copy)]
struct Limits {
    max_checkpoint_copies: usize,
    max_registers: usize,
}

fn retain_candidate(graph: &Graph, copies: &mut Vec<Checkpoint>, limits: Limits) -> bool {
    if copies.len() >= limits.max_checkpoint_copies {
        return false;
    }
    let Some(candidate) = graph.checkpoint_with_limit(limits.max_registers) else {
        return false;
    };
    copies.push(candidate);
    true
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Local example settings: two copies support one acknowledged batch in flight.
    let limits = Limits {
        max_checkpoint_copies: 2,
        max_registers: 128,
    };
    let mut graph = Graph::new();
    graph.add_node("task:7")?;
    graph.set_node_property("task:7", "title", "audit filter")?;

    // Membership and every property count, including deleted registers.
    assert!(graph.checkpoint_with_limit(1).is_none());
    let mut copies = Vec::new();
    assert!(retain_candidate(&graph, &mut copies, limits)); // Bootstrap candidate.
    let mut peer = Graph::from_snapshot(graph.snapshot());
    assert_eq!(peer.snapshot(), graph.snapshot()); // Bootstrap applied and acknowledged.

    graph.set_node_property("task:7", "status", "ready")?;
    assert!(retain_candidate(&graph, &mut copies, limits)); // Pending candidate.
    assert_eq!(copies.len(), limits.max_checkpoint_copies);
    assert!(!retain_candidate(&graph, &mut copies, limits)); // No third copy.

    // A dropped send retries from the retained old acknowledgement.
    let pending = graph.delta_since(&copies[0]);
    assert!(!pending.is_empty());
    // Retry applies the batch before its candidate can become acknowledged.
    peer.apply_delta(&pending)?;
    assert_eq!(peer.snapshot(), graph.snapshot());

    // ACK releases only the old slot; the pending candidate remains the baseline.
    copies.remove(0);
    graph.set_node_property("task:7", "status", "reviewed")?;
    assert!(retain_candidate(&graph, &mut copies, limits));
    assert_eq!(copies.len(), limits.max_checkpoint_copies);

    // Forgetting peer knowledge is safe but requires a full retained-state bootstrap.
    copies.clear();
    let bootstrap = graph.delta_since(&Checkpoint::default());
    let mut restarted = Graph::new();
    assert!(restarted.apply_delta(&bootstrap)?);
    assert_eq!(restarted.snapshot(), graph.snapshot());
    println!("Checkpoint copies stayed bounded; reset peer received complete retained state.");
    Ok(())
}
