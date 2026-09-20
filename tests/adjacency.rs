//! Protect query projections and canonical values while optimizing internal storage.
use zergraph::{EdgeKey, Error, Graph, Snapshot, Value};

const IDS: [&str; 7] = ["", "a", "a\0", "aa", "a:b", "λ", "🕸"];

fn assert_projections(graph: &Graph, extra_ids: &[&str]) {
    let all: Vec<EdgeKey> = graph.edges().map(|edge| edge.key().clone()).collect();
    for id in IDS.into_iter().chain(extra_ids.iter().copied()) {
        let expected_out: Vec<_> = all
            .iter()
            .filter(|edge| edge.source == id)
            .cloned()
            .collect();
        let expected_in: Vec<_> = all
            .iter()
            .filter(|edge| edge.target == id)
            .cloned()
            .collect();
        let actual_out: Vec<_> = graph.outgoing(id).map(|edge| edge.key().clone()).collect();
        let actual_in: Vec<_> = graph.incoming(id).map(|edge| edge.key().clone()).collect();
        assert_eq!(actual_out, expected_out, "outgoing projection for {id:?}");
        assert_eq!(actual_in, expected_in, "incoming projection for {id:?}");
    }
}

fn populated_graph() -> Graph {
    let mut graph = Graph::new();
    // Deliberately insert in reverse order; exposed ordering must come from keys.
    for id in IDS.into_iter().rev() {
        graph.add_node(id).unwrap();
    }
    for source in IDS.into_iter().rev() {
        for target in IDS.into_iter().rev() {
            for label in ["λ", "same\0", "same", ""] {
                graph.add_edge(EdgeKey::new(source, label, target)).unwrap();
            }
        }
    }
    graph
}

#[test]
fn adjacency_matches_ordered_projection_for_all_utf8_keys_labels_and_self_loops() {
    let graph = populated_graph();
    assert_eq!(graph.edges().count(), IDS.len() * IDS.len() * 4);
    assert_projections(&graph, &["missing", "ab", "a\0suffix", "λsuffix"]);
    for id in IDS {
        assert!(graph.edge(&EdgeKey::new(id, "", id)).is_some());
        assert_eq!(graph.outgoing(id).count(), IDS.len() * 4);
        assert_eq!(graph.incoming(id).count(), IDS.len() * 4);
    }
}

#[test]
fn adjacency_tracks_hidden_edges_endpoint_revival_and_explicit_hidden_removal() {
    let mut graph = populated_graph();
    let old = graph.snapshot();
    graph.remove_node("a").unwrap();
    assert_projections(&graph, &["missing"]);
    assert_eq!(graph.outgoing("a").count(), 0);
    assert_eq!(graph.incoming("a").count(), 0);
    let hidden = EdgeKey::new("a", "same", "a\0");
    assert!(graph.remove_edge(&hidden).unwrap());
    assert_projections(&graph, &[]);
    graph.add_node("a").unwrap();
    graph.merge(&old).unwrap();
    assert!(graph.edge(&hidden).is_none());
    assert!(graph.edge(&EdgeKey::new("a", "same", "a")).is_some());
    assert_projections(&graph, &[]);
    graph.add_edge(hidden.clone()).unwrap();
    assert!(graph.edge(&hidden).is_some());
    assert_projections(&graph, &[]);
}

#[test]
fn incoming_and_outgoing_include_new_merged_identities_and_survive_restore() {
    let mut graph = populated_graph();
    let mut peer = graph.fork();
    peer.add_node("remote:new").unwrap();
    peer.add_edge(EdgeKey::new("λ", "from-peer", "remote:new"))
        .unwrap();
    peer.add_edge(EdgeKey::new("remote:new", "from-peer", "a\0"))
        .unwrap();
    peer.add_edge(EdgeKey::new("a", "new-label", "λ")).unwrap();
    peer.remove_node("aa").unwrap();
    graph.merge(&peer.snapshot()).unwrap();
    assert_projections(&graph, &["remote:new", "missing"]);
    assert_eq!(graph.incoming("remote:new").count(), 1);
    assert_eq!(graph.outgoing("remote:new").count(), 1);
    assert!(graph.edge(&EdgeKey::new("a", "new-label", "λ")).is_some());

    let snapshot = Snapshot::from_bytes(&graph.snapshot().to_bytes().unwrap()).unwrap();
    let mut restored = Graph::from_snapshot(snapshot);
    assert_eq!(restored.snapshot(), graph.snapshot());
    assert_projections(&restored, &["remote:new", "missing"]);
    restored.add_node("aa").unwrap();
    assert_projections(&restored, &["remote:new"]);
    graph.merge(&restored.snapshot()).unwrap();
    assert_eq!(graph.snapshot(), restored.snapshot());
    assert_projections(&graph, &["remote:new"]);
}

#[test]
fn rejected_conflicting_merge_does_not_change_adjacency_projections() {
    let mut graph = populated_graph();
    let before = graph.snapshot();
    let mut wire: Value = serde_json::from_slice(&before.to_bytes().unwrap()).unwrap();
    // Forge a different membership value at the identical stamp. The valid
    // unrelated edge below must not leak into an index before this is rejected.
    wire["nodes"][0][1]["live"]["value"] = Value::Bool(false);
    let conflicting = Snapshot::from_bytes(&serde_json::to_vec(&wire).unwrap()).unwrap();
    let mut peer = Graph::from_snapshot(conflicting);
    peer.add_node("remote:new").unwrap();
    peer.add_edge(EdgeKey::new("a", "must-not-leak", "λ"))
        .unwrap();
    peer.add_edge(EdgeKey::new("remote:new", "must-not-leak", "a"))
        .unwrap();
    assert_eq!(graph.merge(&peer.snapshot()), Err(Error::ConflictingStamp));
    assert_eq!(graph.snapshot(), before);
    assert!(graph
        .edge(&EdgeKey::new("a", "must-not-leak", "λ"))
        .is_none());
    assert_projections(&graph, &["remote:new", "missing"]);
}

fn assert_objects_sorted(value: &Value) {
    match value {
        Value::Object(object) => {
            let keys: Vec<_> = object.keys().collect();
            let mut ordered = keys.clone();
            ordered.sort();
            assert_eq!(keys, ordered, "nested object keys must be canonical");
            for child in object.values() {
                assert_objects_sorted(child);
            }
        }
        Value::Array(array) => {
            for child in array {
                assert_objects_sorted(child);
            }
        }
        _ => {}
    }
}

#[test]
fn node_and_edge_setters_canonicalize_owned_nested_values_before_encoding() {
    let mut graph = Graph::new();
    graph.add_node("a").unwrap();
    graph.add_node("b").unwrap();
    let edge = EdgeKey::new("a", "payload", "b");
    graph.add_edge(edge.clone()).unwrap();
    let node_value: Value =
        serde_json::from_str(r#"{"z":1,"a":[{"y":2,"b":{"z":3,"a":4}}]}"#).unwrap();
    let edge_value: Value =
        serde_json::from_str(r#"[{"right":{"z":1,"a":2},"left":[{"z":3,"a":4}]}]"#).unwrap();
    graph.set_node_property("a", "payload", node_value).unwrap();
    graph
        .set_edge_property(&edge, "payload", edge_value)
        .unwrap();
    assert_objects_sorted(graph.node("a").unwrap().property("payload").unwrap());
    assert_objects_sorted(graph.edge(&edge).unwrap().property("payload").unwrap());
    let before = graph.snapshot();
    let bytes = before.to_bytes().unwrap();
    let decoded = Snapshot::from_bytes(&bytes).unwrap();
    assert_eq!(before, decoded);
    assert_eq!(bytes, decoded.to_bytes().unwrap());
    let restored = Graph::from_snapshot(decoded);
    assert_objects_sorted(restored.node("a").unwrap().property("payload").unwrap());
    assert_objects_sorted(restored.edge(&edge).unwrap().property("payload").unwrap());
}
