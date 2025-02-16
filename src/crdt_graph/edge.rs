//! Module for edge-related functionality in the CRDT graph.

/// Represents an edge in the graph.
#[derive(Debug, Clone)]
pub struct Edge {
    pub from: u64,
    pub to: u64,
    // Additional fields to be defined...
}

impl Edge {
    /// Creates a new edge from `from` node to `to` node.
    pub fn new(from: u64, to: u64) -> Self {
        Self { from, to }
    }
} 