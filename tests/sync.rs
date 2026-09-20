use proptest::prelude::*;
use serde_json::{json, Value};
use zergraph::{Checkpoint, Delta, EdgeKey, Error, Graph, Snapshot};

fn pair() -> (Graph, EdgeKey) {
    let mut graph = Graph::new();
    graph.add_node("a").unwrap();
    graph.add_node("b").unwrap();
    graph
        .set_node_property("a", "payload", "large unchanged value")
        .unwrap();
    let edge = EdgeKey::new("a", "links", "b");
    graph.add_edge(edge.clone()).unwrap();
    (graph, edge)
}

fn transport(delta: &Delta) -> Delta {
    Delta::from_bytes(&delta.to_bytes().unwrap()).unwrap()
}

#[test]
fn sparse_delta_contains_only_changed_properties_and_endpoint_membership() {
    let (mut source, edge) = pair();
    let checkpoint = source.checkpoint();
    assert!(source.delta_since(&checkpoint).is_empty());
    source.set_edge_property(&edge, "weight", 2).unwrap();
    let delta = transport(&source.delta_since(&checkpoint));
    let wire: Value = serde_json::from_slice(&delta.to_bytes().unwrap()).unwrap();
    assert_eq!(wire["nodes"].as_array().unwrap().len(), 2);
    assert!(wire["nodes"][0][1]["properties"]
        .as_object()
        .unwrap()
        .is_empty());
    let mut fresh = Graph::new();
    fresh.apply_delta(&delta).unwrap();
    assert_eq!(
        fresh.edge(&edge).unwrap().property("weight"),
        Some(&json!(2))
    );
    assert_eq!(fresh.node("a").unwrap().property("payload"), None);
    fresh.merge(&source.snapshot()).unwrap();
    assert_eq!(fresh.snapshot(), source.snapshot());
}

#[test]
fn deletion_null_revival_and_stale_delivery_match_full_state() {
    let (mut source, edge) = pair();
    let original = source.delta_since(&Checkpoint::default());
    let checkpoint = source.checkpoint();
    source.remove_node_property("a", "payload").unwrap();
    source.set_node_property("a", "null", Value::Null).unwrap();
    source.set_edge_property(&edge, "removed", 1).unwrap();
    source.remove_edge_property(&edge, "removed").unwrap();
    source.remove_edge(&edge).unwrap();
    source.remove_node("b").unwrap();
    let deletion = transport(&source.delta_since(&checkpoint));
    let mut receiver = Graph::new();
    receiver.apply_delta(&deletion).unwrap();
    receiver.apply_delta(&transport(&original)).unwrap();
    assert_eq!(receiver.snapshot(), source.snapshot());
    assert_eq!(
        receiver.node("a").unwrap().property("null"),
        Some(&Value::Null)
    );
    assert!(receiver.node("a").unwrap().property("payload").is_none());
    let before_revival = source.checkpoint();
    source.add_node("b").unwrap();
    source.add_edge(edge.clone()).unwrap();
    receiver
        .apply_delta(&source.delta_since(&before_revival))
        .unwrap();
    assert!(receiver.edge(&edge).unwrap().property("removed").is_none());
    assert_eq!(receiver.snapshot(), source.snapshot());
}

#[test]
fn retry_from_acknowledged_checkpoint_covers_dropped_update() {
    let (mut source, _) = pair();
    let mut receiver = source.fork();
    let acknowledged = source.checkpoint();
    source.set_node_property("a", "first", 1).unwrap();
    let _dropped = source.delta_since(&acknowledged);
    source.set_node_property("b", "second", 2).unwrap();
    let candidate = source.checkpoint();
    receiver
        .apply_delta(&transport(&source.delta_since(&acknowledged)))
        .unwrap();
    // Only now may the sender advance its per-peer knowledge.
    assert_eq!(receiver.snapshot(), source.snapshot());
    assert!(source.delta_since(&candidate).is_empty());
}

#[test]
fn high_counter_on_another_key_does_not_hide_delayed_lower_counter() {
    let (mut origin, _) = pair();
    let initial = origin.checkpoint();
    let mut receiver = origin.fork();
    origin.set_node_property("a", "early", 1).unwrap();
    let early = origin.delta_since(&initial);
    let after_early = origin.checkpoint();
    for n in 0..100 {
        origin.set_node_property("b", "late", n).unwrap();
    }
    receiver
        .apply_delta(&origin.delta_since(&after_early))
        .unwrap();
    let knowledge_with_hole = receiver.checkpoint();
    let missing = origin.delta_since(&knowledge_with_hole);
    assert!(!missing.is_empty());
    receiver.apply_delta(&missing).unwrap();
    assert_eq!(receiver.snapshot(), origin.snapshot());
    assert!(!receiver.apply_delta(&early).unwrap());
}

#[test]
fn reports_include_hidden_and_revived_incident_edges_without_duplicate_ids() {
    let (mut source, forward) = pair();
    let reverse = EdgeKey::new("b", "links", "a");
    let loop_key = EdgeKey::new("a", "self", "a");
    source.add_edge(reverse.clone()).unwrap();
    source.add_edge(loop_key.clone()).unwrap();
    let mut receiver = source.fork();
    let checkpoint = source.checkpoint();
    source.remove_node("a").unwrap();
    let delta = source.delta_since(&checkpoint);
    let report = receiver.apply_delta_with_changes(&delta).unwrap();
    let mut expected = vec![forward, reverse, loop_key];
    expected.sort();
    assert_eq!(report.nodes, ["a"]);
    assert_eq!(report.edges, expected);
    assert_eq!(receiver.edges().count(), 0);
    assert!(receiver
        .apply_delta_with_changes(&delta)
        .unwrap()
        .is_empty());
    source.add_node("a").unwrap();
    let revived = receiver.merge_with_changes(&source.snapshot()).unwrap();
    assert_eq!(revived, report);
    assert_eq!(receiver.edges().count(), 3);
    assert!(receiver
        .merge_with_changes(&source.snapshot())
        .unwrap()
        .is_empty());
}

#[test]
fn report_includes_metadata_updates_even_for_equal_values() {
    let (mut source, _) = pair();
    let mut receiver = source.fork();
    source
        .set_node_property("a", "payload", "large unchanged value")
        .unwrap();
    let changes = receiver.merge_with_changes(&source.snapshot()).unwrap();
    assert_eq!(changes.nodes, ["a"]);
    assert!(changes.edges.is_empty());
}

#[test]
fn wrong_message_kinds_and_malformed_deltas_are_rejected() {
    let (source, _) = pair();
    let delta = source.delta_since(&Checkpoint::default());
    assert!(Snapshot::from_bytes(&delta.to_bytes().unwrap()).is_err());
    assert!(Delta::from_bytes(&source.snapshot().to_bytes().unwrap()).is_err());
    let wire: Value = serde_json::from_slice(&delta.to_bytes().unwrap()).unwrap();
    for mutation in 0..6 {
        let mut bad = wire.clone();
        match mutation {
            0 => {
                bad["nodes"].as_array_mut().unwrap().pop();
            }
            1 => {
                let entry = bad["nodes"][0].clone();
                bad["nodes"].as_array_mut().unwrap().push(entry);
            }
            2 => {
                let entry = bad["edges"][0].clone();
                bad["edges"].as_array_mut().unwrap().push(entry);
            }
            3 => bad["nodes"][0][1]["live"]["stamp"]["counter"] = json!(0),
            4 => bad["kind"] = json!("snapshot"),
            _ => bad["version"] = json!(2),
        }
        assert!(Delta::from_bytes(&serde_json::to_vec(&bad).unwrap()).is_err());
    }
}

#[test]
fn delta_conflict_rejects_all_changes_and_does_not_advance_clock() {
    let (source, _) = pair();
    let before = source.snapshot();
    let mut receiver = source.fork();
    let mut wire: Value = serde_json::from_slice(
        &source
            .delta_since(&Checkpoint::default())
            .to_bytes()
            .unwrap(),
    )
    .unwrap();
    let mut new_record = wire["nodes"][0].clone();
    new_record[0] = json!("00-new");
    new_record[1]["live"]["stamp"]["counter"] = json!(u64::MAX);
    wire["nodes"].as_array_mut().unwrap().insert(0, new_record);
    wire["nodes"][1][1]["properties"]["payload"]["value"]["value"] = json!("conflict");
    let delta = Delta::from_bytes(&serde_json::to_vec(&wire).unwrap()).unwrap();
    assert_eq!(
        receiver.apply_delta_with_changes(&delta),
        Err(Error::ConflictingStamp)
    );
    assert_eq!(receiver.snapshot(), before);
    assert_eq!(receiver.apply_delta(&delta), Err(Error::ConflictingStamp));
    assert!(receiver.add_node("still-writable").unwrap());
}

#[test]
fn restored_writer_can_continue_incremental_exchange() {
    let (mut source, _) = pair();
    let mut restored =
        Graph::from_snapshot(Snapshot::from_bytes(&source.snapshot().to_bytes().unwrap()).unwrap());
    let checkpoint = source.checkpoint();
    source.set_node_property("a", "source", 1).unwrap();
    restored.set_node_property("a", "restored", 2).unwrap();
    let left = source.delta_since(&checkpoint);
    let right = restored.delta_since(&checkpoint);
    source.apply_delta(&right).unwrap();
    restored.apply_delta(&left).unwrap();
    assert_eq!(source.snapshot(), restored.snapshot());
}

#[test]
fn delta_comparison_handles_shifted_and_missing_checkpoint_keys() {
    let (mut source, _) = pair();
    let mut peer = source.fork();
    peer.add_node("0-peer-only").unwrap();
    let checkpoint = peer.checkpoint();
    source.add_node("00-source-only").unwrap();
    source.add_node("z-source-only").unwrap();
    source
        .add_edge(EdgeKey::new("00-source-only", "link", "a"))
        .unwrap();
    source.set_node_property("b", "new", 7).unwrap();
    let mut expected = peer.fork();
    expected.merge(&source.snapshot()).unwrap();
    peer.apply_delta(&transport(&source.delta_since(&checkpoint)))
        .unwrap();
    assert_eq!(peer.snapshot(), expected.snapshot());
    assert!(source.delta_since(&peer.checkpoint()).is_empty());
}

#[test]
fn checkpoint_limits_include_hidden_membership_and_property_tombstones() {
    let empty = Graph::new();
    assert!(empty.checkpoint_with_limit(0).is_some());
    let (mut graph, edge) = pair();
    graph.set_edge_property(&edge, "cost", 1).unwrap();
    // Two nodes, one edge, and two properties are five retained registers.
    assert!(graph.checkpoint_with_limit(4).is_none());
    let exact = graph.checkpoint_with_limit(5).unwrap();
    assert!(graph.delta_since(&exact).is_empty());
    graph.remove_node_property("a", "payload").unwrap();
    graph.remove_edge_property(&edge, "cost").unwrap();
    graph.remove_edge(&edge).unwrap();
    graph.remove_node("b").unwrap();
    let before = graph.snapshot();
    assert!(graph.checkpoint_with_limit(4).is_none());
    assert_eq!(graph.snapshot(), before);
    let retained = graph.checkpoint_with_limit(5).unwrap();
    assert!(graph.delta_since(&retained).is_empty());
    graph.add_node("b").unwrap();
    assert!(!graph.delta_since(&retained).is_empty());
}

proptest! {
    #[test]
    fn shuffled_duplicate_batches_equal_full_merge(history in prop::collection::vec((0_u8..3, 0_u8..9, any::<u8>()), 1..70)) {
        let (base, edge) = pair();
        let mut replicas = [base.fork(), base.fork(), base.fork()];
        let mut pending = vec![(0, base.delta_since(&Checkpoint::default()))];
        for (actor, operation, priority) in history {
            let g = &mut replicas[actor as usize];
            let checkpoint = g.checkpoint();
            match operation {
                0 => { g.add_node("a").unwrap(); }
                1 => { g.remove_node("a").unwrap(); }
                2 => { let _ = g.set_node_property("a", "x", priority as i64); }
                3 => { let _ = g.remove_node_property("a", "x"); }
                4 => { let _ = g.add_edge(edge.clone()); }
                5 => { g.remove_edge(&edge).unwrap(); }
                6 => { let _ = g.set_edge_property(&edge, "x", Value::Null); }
                7 => { let _ = g.remove_edge_property(&edge, "x"); }
                _ => { g.set_node_property("b", "y", priority as i64).unwrap(); }
            }
            pending.push((priority, transport(&g.delta_since(&checkpoint))));
        }
        let mut expected = base.fork();
        for g in replicas { expected.merge(&g.snapshot()).unwrap(); }
        pending.sort_by_key(|(priority, _)| *priority);
        let mut receiver = Graph::new();
        for (_, delta) in pending.into_iter().rev() {
            receiver.apply_delta_with_changes(&delta).unwrap();
            prop_assert!(receiver.apply_delta_with_changes(&delta).unwrap().is_empty());
        }
        prop_assert_eq!(receiver.snapshot(), expected.snapshot());
    }
}
