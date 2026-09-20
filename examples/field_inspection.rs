//! Static asset and observation fixtures, without devices or external services.
use zergraph::{EdgeKey, Graph, Snapshot, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut graph = Graph::new();
    graph.add_node("asset:ahu-17")?;
    graph.add_node("observation:2026-09-19:ahu-17")?;
    graph.set_node_property("asset:ahu-17", "kind", "air-handler")?;
    graph.set_node_property("observation:2026-09-19:ahu-17", "temperature_c", 31.2)?;
    graph.set_node_property("observation:2026-09-19:ahu-17", "draft_note", "check units")?;
    let observation = EdgeKey::new("observation:2026-09-19:ahu-17", "observes", "asset:ahu-17");
    graph.add_edge(observation.clone())?;

    let mut inspector = graph.fork();
    let mut planner = graph.fork();
    inspector.set_node_property("observation:2026-09-19:ahu-17", "temperature_c", 21.3)?;
    inspector.remove_node_property("observation:2026-09-19:ahu-17", "draft_note")?;
    inspector.set_node_property(
        "observation:2026-09-19:ahu-17",
        "reviewed_by",
        "inspector-a",
    )?;
    planner.set_node_property("asset:ahu-17", "next_inspection", "2026-10-19")?;
    // A known null value is different from an absent/removed property.
    planner.set_node_property("asset:ahu-17", "work_order", Value::Null)?;

    graph.merge(&inspector.snapshot())?;
    graph.merge(&planner.snapshot())?;
    let restored = Graph::from_snapshot(Snapshot::from_bytes(&graph.snapshot().to_bytes()?)?);
    let reading = restored.node("observation:2026-09-19:ahu-17").unwrap();
    assert_eq!(reading.property("temperature_c"), Some(&Value::from(21.3)));
    assert!(reading.property("draft_note").is_none());
    assert_eq!(
        reading.property("reviewed_by"),
        Some(&Value::from("inspector-a"))
    );
    let asset = restored.node("asset:ahu-17").unwrap();
    assert_eq!(
        asset.property("next_inspection"),
        Some(&Value::from("2026-10-19"))
    );
    assert_eq!(asset.property("work_order"), Some(&Value::Null));
    assert!(restored.edge(&observation).is_some());
    assert_eq!(restored.nodes().count(), 2);
    assert_eq!(restored.edges().count(), 1);
    println!("Correction retained: AHU-17 observation is 21.3 C.");
    println!("Independent inspection schedule and observation relationship survive restore.");
    Ok(())
}
