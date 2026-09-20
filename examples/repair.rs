//! Conflicting repair observations remain separate evidence after restore.
use zergraph::{EdgeKey, Graph, Snapshot, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut base = Graph::new();
    for id in ["part:motor-77", "machine:lathe-2"] {
        base.add_node(id)?;
    }
    base.add_edge(EdgeKey::new(
        "part:motor-77",
        "candidate-for",
        "machine:lathe-2",
    ))?;

    let mut shop_a = base.fork();
    let mut shop_b = base.fork();
    for (shop, assertion, verdict, evidence) in [
        (&mut shop_a, "assertion:shop-a:fit", "fits", "jig-a"),
        (&mut shop_b, "assertion:shop-b:fit", "does-not-fit", "jig-b"),
    ] {
        shop.add_node(assertion)?;
        shop.set_node_property(assertion, "verdict", verdict)?;
        shop.set_node_property(assertion, "evidence", evidence)?;
        shop.add_edge(EdgeKey::new(assertion, "tests", "part:motor-77"))?;
    }

    base.merge(&shop_a.snapshot())?;
    base.merge(&shop_b.snapshot())?;
    let merged = base.snapshot();
    let restored = Graph::from_snapshot(Snapshot::from_bytes(&merged.to_bytes()?)?);

    assert_eq!(restored.snapshot(), merged);
    assert!(restored
        .edge(&EdgeKey::new(
            "part:motor-77",
            "candidate-for",
            "machine:lathe-2"
        ))
        .is_some());
    let verdicts: Vec<_> = restored
        .incoming("part:motor-77")
        .filter(|edge| edge.key().label == "tests")
        .map(|edge| edge.key().source.clone())
        .collect();
    assert_eq!(
        verdicts,
        vec![
            "assertion:shop-a:fit".to_string(),
            "assertion:shop-b:fit".to_string(),
        ]
    );
    assert_eq!(
        restored
            .node("assertion:shop-a:fit")
            .unwrap()
            .property("verdict"),
        Some(&Value::from("fits"))
    );
    assert_eq!(
        restored
            .node("assertion:shop-b:fit")
            .unwrap()
            .property("verdict"),
        Some(&Value::from("does-not-fit"))
    );
    println!("Both repair verdicts survive; a person can inspect their evidence.");
    Ok(())
}
