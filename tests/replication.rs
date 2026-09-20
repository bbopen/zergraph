//! Observable graph semantics and merge laws over reachable replica histories.
use proptest::prelude::*;
use zergraph::{EdgeKey, Graph, Snapshot, Value};

fn pair_graph() -> (Graph, EdgeKey) {
    let mut graph = Graph::new();
    graph.add_node("a").unwrap();
    graph.add_node("b").unwrap();
    let edge = EdgeKey::new("a", "knows", "b");
    graph.add_edge(edge.clone()).unwrap();
    (graph, edge)
}

fn join(left: &Snapshot, right: &Snapshot) -> Snapshot {
    let mut graph = Graph::from_snapshot(left.clone());
    graph.merge(right).unwrap();
    graph.snapshot()
}

#[test]
fn concurrent_property_writes_converge_in_both_directions() {
    let (mut left, _) = pair_graph();
    let mut right = left.fork();
    left.set_node_property("a", "name", "left").unwrap();
    right.set_node_property("a", "name", "right").unwrap();
    let left_before = left.snapshot();
    let right_before = right.snapshot();
    left.merge(&right_before).unwrap();
    right.merge(&left_before).unwrap();
    assert_eq!(left.snapshot(), right.snapshot());
    let node = left.node("a").unwrap();
    let winner = node.property("name").unwrap();
    assert!(winner == &Value::from("left") || winner == &Value::from("right"));
}

#[test]
fn concurrent_updates_to_different_properties_are_all_retained() {
    let (mut left, edge) = pair_graph();
    let mut right = left.fork();
    left.set_node_property("a", "left", 1_i64).unwrap();
    right.set_node_property("a", "right", 2_i64).unwrap();
    left.set_edge_property(&edge, "left", 3_i64).unwrap();
    right.set_edge_property(&edge, "right", 4_i64).unwrap();
    left.merge(&right.snapshot()).unwrap();
    assert_eq!(
        left.node("a").unwrap().property("left"),
        Some(&Value::from(1_i64))
    );
    assert_eq!(
        left.node("a").unwrap().property("right"),
        Some(&Value::from(2_i64))
    );
    assert_eq!(
        left.edge(&edge).unwrap().property("left"),
        Some(&Value::from(3_i64))
    );
    assert_eq!(
        left.edge(&edge).unwrap().property("right"),
        Some(&Value::from(4_i64))
    );
}

#[test]
fn write_after_observing_another_replica_wins() {
    let (mut left, _) = pair_graph();
    let mut right = left.fork();
    for value in 0..25_i64 {
        left.set_node_property("a", "version", value).unwrap();
    }
    right.merge(&left.snapshot()).unwrap();
    right.set_node_property("a", "version", 100_i64).unwrap();
    left.merge(&right.snapshot()).unwrap();
    assert_eq!(
        left.node("a").unwrap().property("version"),
        Some(&Value::from(100_i64))
    );
}

#[test]
fn node_deletion_hides_concurrent_incident_edge_and_revival_restores_it() {
    let mut remover = Graph::new();
    remover.add_node("a").unwrap();
    remover.add_node("b").unwrap();
    remover
        .set_node_property("b", "name", "same entity")
        .unwrap();
    let mut creator = remover.fork();
    remover.remove_node("b").unwrap();
    let edge = EdgeKey::new("a", "knows", "b");
    creator.add_edge(edge.clone()).unwrap();
    creator
        .set_edge_property(&edge, "source", "concurrent")
        .unwrap();
    let removed = remover.snapshot();
    let created = creator.snapshot();
    remover.merge(&created).unwrap();
    creator.merge(&removed).unwrap();
    assert_eq!(remover.snapshot(), creator.snapshot());
    assert!(remover.node("b").is_none());
    assert!(remover.edge(&edge).is_none());
    assert_eq!(remover.edges().count(), 0);
    assert_eq!(remover.outgoing("a").count(), 0);
    assert_eq!(remover.incoming("b").count(), 0);
    remover.add_node("b").unwrap();
    assert_eq!(
        remover.node("b").unwrap().property("name"),
        Some(&Value::from("same entity"))
    );
    assert_eq!(
        remover.edge(&edge).unwrap().property("source"),
        Some(&Value::from("concurrent"))
    );
}

#[test]
fn removing_an_edge_survives_endpoint_revival_and_stale_snapshot_replay() {
    let (mut graph, edge) = pair_graph();
    let stale = graph.snapshot();
    graph.remove_edge(&edge).unwrap();
    graph.remove_node("b").unwrap();
    graph.add_node("b").unwrap();
    graph.merge(&stale).unwrap();
    assert!(graph.node("b").is_some());
    assert!(graph.edge(&edge).is_none());
}

#[test]
fn hidden_property_update_does_not_resurrect_node_but_survives_revival() {
    let (mut remover, _) = pair_graph();
    let mut updater = remover.fork();
    remover.remove_node("b").unwrap();
    updater
        .set_node_property("b", "knowledge", "learned concurrently")
        .unwrap();
    remover.merge(&updater.snapshot()).unwrap();
    assert!(remover.node("b").is_none());
    remover.add_node("b").unwrap();
    assert_eq!(
        remover.node("b").unwrap().property("knowledge"),
        Some(&Value::from("learned concurrently"))
    );
}

#[test]
fn structured_edge_keys_preserve_colons_and_distinct_labels() {
    let mut graph = Graph::new();
    for id in ["a:b", "c", "a", "b:c"] {
        graph.add_node(id).unwrap();
    }
    let first = EdgeKey::new("a:b", "related:to", "c");
    let second = EdgeKey::new("a", "related:to", "b:c");
    let third = EdgeKey::new("a", "different", "b:c");
    for (edge, value) in [(&first, "first"), (&second, "second"), (&third, "third")] {
        graph.add_edge(edge.clone()).unwrap();
        graph.set_edge_property(edge, "name", value).unwrap();
    }
    assert_eq!(graph.edges().count(), 3);
    assert_eq!(
        graph.edge(&first).unwrap().property("name"),
        Some(&Value::from("first"))
    );
    graph.remove_edge(&second).unwrap();
    assert!(graph.edge(&first).is_some());
    assert!(graph.edge(&second).is_none());
    assert!(graph.edge(&third).is_some());
    let bytes = graph.snapshot().to_bytes().unwrap();
    let restored = Graph::from_snapshot(Snapshot::from_bytes(&bytes).unwrap());
    assert_eq!(graph.snapshot(), restored.snapshot());
}

#[test]
fn property_deletion_is_distinct_from_null_and_survives_stale_merge() {
    let (mut graph, edge) = pair_graph();
    graph
        .set_node_property("a", "nullable", Value::Null)
        .unwrap();
    graph
        .set_edge_property(&edge, "nullable", Value::Null)
        .unwrap();
    assert_eq!(
        graph.node("a").unwrap().property("nullable"),
        Some(&Value::Null)
    );
    assert_eq!(
        graph.edge(&edge).unwrap().property("nullable"),
        Some(&Value::Null)
    );
    let stale = graph.snapshot();
    graph.remove_node_property("a", "nullable").unwrap();
    graph.remove_edge_property(&edge, "nullable").unwrap();
    graph.merge(&stale).unwrap();
    assert!(graph.node("a").unwrap().property("nullable").is_none());
    assert!(graph.edge(&edge).unwrap().property("nullable").is_none());
    let mut restored =
        Graph::from_snapshot(Snapshot::from_bytes(&graph.snapshot().to_bytes().unwrap()).unwrap());
    restored.merge(&stale).unwrap();
    assert_eq!(graph.snapshot(), restored.snapshot());
}

#[test]
fn snapshot_roundtrip_keeps_hidden_state_and_can_resume_writing() {
    let (mut graph, edge) = pair_graph();
    graph.set_edge_property(&edge, "retained", "yes").unwrap();
    graph.set_node_property("b", "removed", "old").unwrap();
    let stale = graph.snapshot();
    graph.remove_node_property("b", "removed").unwrap();
    graph.remove_node("b").unwrap();
    let snapshot = graph.snapshot();
    let mut restored =
        Graph::from_snapshot(Snapshot::from_bytes(&snapshot.to_bytes().unwrap()).unwrap());
    assert_eq!(snapshot, restored.snapshot());
    restored.merge(&stale).unwrap();
    assert!(restored.node("b").is_none());
    restored.add_node("b").unwrap();
    assert!(restored.node("b").unwrap().property("removed").is_none());
    assert_eq!(
        restored.edge(&edge).unwrap().property("retained"),
        Some(&Value::from("yes"))
    );
    restored.set_node_property("b", "fresh", true).unwrap();
    graph.merge(&restored.snapshot()).unwrap();
    assert_eq!(graph.snapshot(), restored.snapshot());
}

#[test]
fn independently_resumed_snapshots_have_distinct_writer_identity() {
    let (graph, _) = pair_graph();
    let mut first = Graph::from_snapshot(graph.snapshot());
    let mut second = Graph::from_snapshot(graph.snapshot());
    first.set_node_property("a", "same-key", "first").unwrap();
    second.set_node_property("a", "same-key", "second").unwrap();
    assert_eq!(
        join(&first.snapshot(), &second.snapshot()),
        join(&second.snapshot(), &first.snapshot())
    );
}

#[test]
fn rejected_writes_do_not_change_state() {
    let (mut graph, edge) = pair_graph();
    graph.remove_node("b").unwrap();
    let before = graph.snapshot();
    assert!(graph.set_node_property("b", "x", "invalid").is_err());
    assert!(graph.set_edge_property(&edge, "x", "invalid").is_err());
    assert!(graph
        .add_edge(EdgeKey::new("a", "other", "missing"))
        .is_err());
    assert_eq!(before, graph.snapshot());
}

#[test]
fn duplicate_additions_preserve_properties_and_adjacency() {
    let (mut graph, edge) = pair_graph();
    graph.set_node_property("a", "name", "retained").unwrap();
    graph
        .set_edge_property(&edge, "name", "also retained")
        .unwrap();
    let before = graph.snapshot();
    assert!(!graph.add_node("a").unwrap());
    assert!(!graph.add_edge(edge.clone()).unwrap());
    assert_eq!(before, graph.snapshot());
    assert_eq!(graph.outgoing("a").count(), 1);
    assert_eq!(graph.incoming("b").count(), 1);
}

#[test]
fn iteration_and_snapshot_encoding_are_stable_across_merge_direction() {
    let mut left = Graph::new();
    for id in ["z", "a", "m"] {
        left.add_node(id).unwrap();
    }
    let mut right = left.fork();
    left.add_edge(EdgeKey::new("z", "second", "m")).unwrap();
    right.add_edge(EdgeKey::new("a", "first", "z")).unwrap();
    let left_before = left.snapshot();
    let right_before = right.snapshot();
    left.merge(&right_before).unwrap();
    right.merge(&left_before).unwrap();
    assert_eq!(
        left.nodes()
            .map(|node| node.id().to_owned())
            .collect::<Vec<_>>(),
        vec!["a", "m", "z"]
    );
    let keys = left
        .edges()
        .map(|edge| edge.key().clone())
        .collect::<Vec<_>>();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted);
    assert_eq!(
        left.snapshot().to_bytes().unwrap(),
        right.snapshot().to_bytes().unwrap()
    );
}

// Histories contain legal local mutations, forked writers, and state transfers.
// The assertions concern the complete replicated state, including hidden facts
// and tombstones, rather than only today's visible graph.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]
    #[test]
    fn merge_laws_hold_after_generated_reachable_histories(
        actions in prop::collection::vec((0usize..3, 0u8..10, 0usize..3, 0usize..3, -20i64..20), 0..90)
    ) {
        let mut first = Graph::new();
        for id in ["a", "b", "c"] { first.add_node(id).unwrap(); }
        let second = first.fork(); let third = first.fork();
        let mut replicas = [first, second, third];
        let ids = ["a", "b", "c"];
        for (writer, op, source, target, value) in actions {
            if op == 8 {
                let remote = replicas[(writer + 1) % 3].snapshot();
                replicas[writer].merge(&remote).unwrap();
                continue;
            }
            if op == 9 {
                let bytes = replicas[writer].snapshot().to_bytes().unwrap();
                replicas[writer] = Graph::from_snapshot(Snapshot::from_bytes(&bytes).unwrap());
                continue;
            }
            let graph = &mut replicas[writer];
            let id = ids[source];
            let edge = EdgeKey::new(id, "related", ids[target]);
            match op {
                0 => { graph.add_node(id).unwrap(); }
                1 => { graph.remove_node(id).unwrap(); }
                2 if graph.node(id).is_some() => { graph.set_node_property(id, "value", value).unwrap(); }
                3 if graph.node(id).is_some() => { graph.remove_node_property(id, "value").unwrap(); }
                4 if graph.node(id).is_some() && graph.node(ids[target]).is_some() => { graph.add_edge(edge).unwrap(); }
                5 => { graph.remove_edge(&edge).unwrap(); }
                6 if graph.edge(&edge).is_some() => { graph.set_edge_property(&edge, "value", value).unwrap(); }
                7 if graph.edge(&edge).is_some() => { graph.remove_edge_property(&edge, "value").unwrap(); }
                _ => {}
            }
        }
        let a = replicas[0].snapshot(); let b = replicas[1].snapshot(); let c = replicas[2].snapshot();
        prop_assert_eq!(join(&a, &a), a.clone(), "idempotence");
        prop_assert_eq!(join(&a, &b), join(&b, &a), "commutativity");
        prop_assert_eq!(join(&join(&a, &b), &c), join(&a, &join(&b, &c)), "associativity");
        let expected = join(&join(&a, &b), &c);
        for replica in &mut replicas {
            replica.merge(&c).unwrap(); replica.merge(&a).unwrap(); replica.merge(&b).unwrap();
            prop_assert_eq!(replica.snapshot(), expected.clone(), "all writers converge");
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, failure_persistence: None, ..ProptestConfig::default() })]
    #[test]
    fn finite_json_numbers_survive_snapshot_roundtrip_without_changing_stamped_value(value in any::<f64>().prop_filter("finite JSON number", |n| n.is_finite())) {
        let mut graph = Graph::new();
        graph.add_node("n").unwrap();
        graph.set_node_property("n", "number", value).unwrap();
        let original = graph.snapshot();
        let decoded = Snapshot::from_bytes(&original.to_bytes().unwrap()).unwrap();
        prop_assert_eq!(&original, &decoded);
        // A transport round-trip must never create a same-stamp conflict.
        graph.merge(&decoded).unwrap();
    }
}

#[test]
fn exact_float_regression_and_extremes_survive_transport() {
    for number in [
        5.139324861199435e-197,
        -0.0,
        f64::MIN_POSITIVE,
        f64::MAX,
        f64::from_bits(1),
    ] {
        let mut graph = Graph::new();
        graph.add_node("n").unwrap();
        graph.set_node_property("n", "number", number).unwrap();
        let snapshot = graph.snapshot();
        let decoded = Snapshot::from_bytes(&snapshot.to_bytes().unwrap()).unwrap();
        assert_eq!(
            snapshot, decoded,
            "changed floating-point property {number}"
        );
        assert!(!graph.merge(&decoded).unwrap());
    }
}
