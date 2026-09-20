//! A local-only swarm fixture: records are observations, not robot commands.
use std::thread;
use zergraph::{EdgeKey, Error, Graph, Snapshot, Value};

fn observe(
    mut robot: Graph,
    robot_id: &str,
    temperature_c: i64,
    disconnected_note: bool,
) -> Result<Snapshot, Error> {
    let observation = format!("observation:{robot_id}:shared-asset");
    robot.add_node(&observation)?;
    robot.set_node_property(&observation, "robot", robot_id)?;
    robot.set_node_property(&observation, "temperature_c", temperature_c)?;
    robot.add_edge(EdgeKey::new(&observation, "observes", "asset:shared"))?;

    if disconnected_note {
        let note = format!("note:{robot_id}:offline");
        robot.add_node(&note)?;
        robot.set_node_property(&note, "text", "battery checked while disconnected")?;
    }
    Ok(robot.snapshot())
}

fn deliver(graph: &mut Graph, packets: &[&Snapshot]) -> Result<(), Error> {
    for packet in packets {
        graph.merge(packet)?;
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut base = Graph::new();
    base.add_node("asset:shared")?;
    base.set_node_property("asset:shared", "kind", "inspection target")?;

    // Each fork gets a distinct writer identity before its local work begins.
    let [r1, r2, r3, r4] = [base.fork(), base.fork(), base.fork(), base.fork()];
    let [one, two, three, four] = thread::scope(|scope| {
        let one = scope.spawn(move || observe(r1, "robot-1", 20, false));
        let two = scope.spawn(move || observe(r2, "robot-2", 27, false));
        let three = scope.spawn(move || observe(r3, "robot-3", 20, false));
        let four = scope.spawn(move || observe(r4, "robot-4", 24, true));
        [
            one.join().expect("robot-1 thread panicked"),
            two.join().expect("robot-2 thread panicked"),
            three.join().expect("robot-3 thread panicked"),
            four.join().expect("robot-4 thread panicked"),
        ]
    });
    let [one, two, three, four] = [one?, two?, three?, four?];

    // Complete snapshots arrive after different delays, with duplicates and order changes.
    let mut a = base.fork();
    let mut b = base.fork();
    let mut c = base.fork();
    let mut d = base.fork();
    deliver(&mut a, &[&three, &one, &three, &four, &two])?;
    deliver(&mut b, &[&two, &four, &one, &two, &three])?;
    deliver(&mut c, &[&four, &three, &two, &one, &four])?;
    deliver(&mut d, &[&one, &two, &four, &three, &one])?;

    let expected = a.snapshot();
    assert_eq!(b.snapshot(), expected);
    assert_eq!(c.snapshot(), expected);
    assert_eq!(d.snapshot(), expected);

    // Separate observation IDs preserve the disagreement as evidence.
    for (robot, temperature) in [
        ("robot-1", 20_i64),
        ("robot-2", 27_i64),
        ("robot-3", 20_i64),
        ("robot-4", 24_i64),
    ] {
        let observation = format!("observation:{robot}:shared-asset");
        assert_eq!(
            a.node(&observation).unwrap().property("temperature_c"),
            Some(&Value::from(temperature))
        );
        assert!(a
            .edge(&EdgeKey::new(observation, "observes", "asset:shared"))
            .is_some());
    }
    assert_eq!(
        a.node("note:robot-4:offline").unwrap().property("text"),
        Some(&Value::from("battery checked while disconnected"))
    );

    let bytes = expected.to_bytes()?;
    let restored = Graph::from_snapshot(Snapshot::from_bytes(&bytes)?);
    assert_eq!(restored.snapshot(), expected);
    println!("Four local observation replicas converged and restored exactly.");
    Ok(())
}
