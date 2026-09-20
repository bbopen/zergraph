//! Independent lineage facts converge and remain inspectable after restore.
use zergraph::{EdgeKey, Graph, Snapshot, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut base = Graph::new();
    for id in [
        "dataset:water-2026-09",
        "script:train-7",
        "model:algae-3",
        "evaluation:holdout-4",
    ] {
        base.add_node(id)?;
    }
    base.set_node_property("dataset:water-2026-09", "sha256", "data-hash")?;
    base.set_node_property("script:train-7", "commit", "abc123")?;

    let mut trainer = base.fork();
    let mut evaluator = base.fork();
    trainer.add_edge(EdgeKey::new(
        "model:algae-3",
        "trained-on",
        "dataset:water-2026-09",
    ))?;
    trainer.add_edge(EdgeKey::new("model:algae-3", "uses", "script:train-7"))?;
    evaluator.add_edge(EdgeKey::new(
        "evaluation:holdout-4",
        "evaluates",
        "model:algae-3",
    ))?;
    evaluator.set_node_property("evaluation:holdout-4", "accuracy", 0.91)?;

    base.merge(&trainer.snapshot())?;
    base.merge(&evaluator.snapshot())?;
    let merged = base.snapshot();
    let restored = Graph::from_snapshot(Snapshot::from_bytes(&merged.to_bytes()?)?);

    assert_eq!(restored.snapshot(), merged);
    assert_eq!(
        restored
            .node("evaluation:holdout-4")
            .unwrap()
            .property("accuracy"),
        Some(&Value::from(0.91))
    );
    assert!(restored.outgoing("model:algae-3").any(|edge| {
        edge.key().label == "trained-on" && edge.key().target == "dataset:water-2026-09"
    }));
    assert!(restored
        .edge(&EdgeKey::new("model:algae-3", "uses", "script:train-7"))
        .is_some());
    assert!(restored.incoming("model:algae-3").any(|edge| {
        edge.key().label == "evaluates" && edge.key().source == "evaluation:holdout-4"
    }));
    println!("Holdout-4 is linked to algae-3, its data, and its training script.");
    Ok(())
}
