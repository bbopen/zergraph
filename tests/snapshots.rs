use serde_json::{json, Value};
use zergraph::{EdgeKey, Error, Graph, Snapshot};

fn fixture() -> Value {
    let mut graph = Graph::new();
    graph.add_node("a").unwrap();
    graph.add_node("b").unwrap();
    graph.add_edge(EdgeKey::new("a", "links", "b")).unwrap();
    graph.set_node_property("a", "answer", 42).unwrap();
    serde_json::from_slice(&graph.snapshot().to_bytes().unwrap()).unwrap()
}

fn decode(value: &Value) -> Result<Snapshot, Error> {
    Snapshot::from_bytes(&serde_json::to_vec(value).unwrap())
}

#[test]
fn malformed_and_unsupported_snapshots_are_errors() {
    assert!(Snapshot::from_bytes(b"not JSON").is_err());
    assert!(Snapshot::from_bytes(b"{}").is_err());
    let mut wire = fixture();
    wire["version"] = json!(2);
    assert_eq!(decode(&wire), Err(Error::UnsupportedVersion(2)));
}

#[test]
fn duplicate_node_or_edge_identity_is_not_silently_overwritten() {
    for field in ["nodes", "edges"] {
        let mut wire = fixture();
        let entries = wire[field].as_array_mut().unwrap();
        entries.push(entries[0].clone());
        assert!(matches!(decode(&wire), Err(Error::InvalidSnapshot(_))));
    }
}

#[test]
fn absent_endpoint_records_are_rejected_but_deleted_endpoints_survive() {
    let mut wire = fixture();
    wire["nodes"].as_array_mut().unwrap().pop();
    assert!(decode(&wire).is_err());
    let mut graph = Graph::from_snapshot(decode(&fixture()).unwrap());
    graph.remove_node("b").unwrap();
    let snapshot = Snapshot::from_bytes(&graph.snapshot().to_bytes().unwrap()).unwrap();
    let mut restored = Graph::from_snapshot(snapshot);
    assert_eq!(restored.edges().count(), 0);
    restored.add_node("b").unwrap();
    assert_eq!(restored.edges().count(), 1);
}

#[test]
fn invalid_stamps_are_rejected_at_decode_boundary() {
    let mut wire = fixture();
    wire["nodes"][0][1]["live"]["stamp"]["counter"] = json!(0);
    assert!(decode(&wire).is_err());
    let mut wire = fixture();
    wire["nodes"][0][1]["live"]["stamp"]["writer"] = json!("00000000-0000-0000-0000-000000000000");
    assert!(decode(&wire).is_err());
}

#[test]
fn exhausted_clock_fails_without_changing_graph() {
    let mut wire = fixture();
    wire["nodes"][0][1]["live"]["stamp"]["counter"] = json!(u64::MAX);
    let mut graph = Graph::from_snapshot(decode(&wire).unwrap());
    let before = graph.snapshot();
    assert_eq!(graph.add_node("c"), Err(Error::ClockExhausted));
    assert_eq!(graph.snapshot(), before);
    assert_eq!(graph.remove_node("a"), Err(Error::ClockExhausted));
    assert_eq!(graph.snapshot(), before);
    assert_eq!(
        graph.set_node_property("a", "answer", 0),
        Err(Error::ClockExhausted)
    );
    assert_eq!(graph.snapshot(), before);
}

#[test]
fn conflicting_stamp_merge_is_atomic_including_unrelated_new_nodes() {
    let wire = fixture();
    let mut graph = Graph::from_snapshot(decode(&wire).unwrap());
    let before = graph.snapshot();
    let mut changed = wire;
    changed["nodes"][0][1]["properties"]["answer"]["value"] = json!({"kind":"value", "value":99});
    let mut malicious = Graph::from_snapshot(decode(&changed).unwrap());
    malicious.add_node("00-before-conflict").unwrap();
    assert_eq!(
        graph.merge(&malicious.snapshot()),
        Err(Error::ConflictingStamp)
    );
    assert_eq!(graph.snapshot(), before);
}

#[test]
fn owned_nested_json_and_large_integers_survive_snapshot() {
    let mut graph = Graph::new();
    graph.add_node("n").unwrap();
    let payload =
        json!({"list":[true, null, "λ"], "max": u64::MAX, "min": i64::MIN, "nested":{"x":1}});
    graph
        .set_node_property("n", "payload", payload.clone())
        .unwrap();
    let restored =
        Graph::from_snapshot(Snapshot::from_bytes(&graph.snapshot().to_bytes().unwrap()).unwrap());
    assert_eq!(
        restored.node("n").unwrap().property("payload"),
        Some(&payload)
    );
    assert_eq!(restored.snapshot(), graph.snapshot());
}

#[test]
fn restore_can_edit_and_merge_with_the_original_live_writer() {
    let mut original = Graph::from_snapshot(decode(&fixture()).unwrap());
    let mut restored = Graph::from_snapshot(
        Snapshot::from_bytes(&original.snapshot().to_bytes().unwrap()).unwrap(),
    );
    original.set_node_property("a", "original", true).unwrap();
    restored.set_node_property("a", "restored", true).unwrap();
    let left = original.snapshot();
    original.merge(&restored.snapshot()).unwrap();
    restored.merge(&left).unwrap();
    assert_eq!(original.snapshot(), restored.snapshot());
    assert_eq!(
        original.node("a").unwrap().property("original"),
        Some(&json!(true))
    );
    assert_eq!(
        original.node("a").unwrap().property("restored"),
        Some(&json!(true))
    );
}

#[test]
fn nested_object_insertion_order_does_not_change_snapshot_bytes() {
    let mut wire = fixture();
    wire["nodes"][0][1]["properties"]["answer"]["value"]["value"] =
        serde_json::from_str::<Value>(r#"{"z":1,"a":[{"z":2,"a":3}]}"#).unwrap();
    let first = decode(&wire).unwrap();
    wire["nodes"][0][1]["properties"]["answer"]["value"]["value"] =
        serde_json::from_str::<Value>(r#"{"a":[{"a":3,"z":2}],"z":1}"#).unwrap();
    let second = decode(&wire).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.to_bytes().unwrap(), second.to_bytes().unwrap());
}
