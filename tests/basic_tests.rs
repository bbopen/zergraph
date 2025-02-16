#![cfg(test)]

use zergraph::crdt_graph::node::Node;

#[test]
fn test_node_new() {
    let node = Node::new(42);
    assert_eq!(node.id, 42);
} 