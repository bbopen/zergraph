//! A small, synchronous, mergeable property graph.
//!
//! Nodes have caller-chosen string IDs. Edges are identified by their complete
//! `(source, label, target)` key. Properties are owned JSON values. Every writer
//! gets a fresh random identity; fork or restore a snapshot to create another
//! writer. There is deliberately no `Clone` implementation for a live graph.
//!
//! Membership and each property use independent last-write-wins registers,
//! ordered by `(logical counter, writer UUID)`. This is a logical order, not wall
//! time. Removing a node hides its incident edges. Re-adding that same ID revives
//! its previous properties and any still-live edges; use a fresh ID for a new
//! entity. Snapshots retain hidden state and deletion metadata.
//!
//! ```
//! use zergraph::{EdgeKey, Graph, Snapshot};
//! let mut a = Graph::new();
//! a.add_node("change:42")?;
//! a.add_node("test:17")?;
//! let mut b = a.fork();
//! a.add_edge(EdgeKey::new("change:42", "validated-by", "test:17"))?;
//! b.set_node_property("test:17", "passed", true)?;
//! a.merge(&b.snapshot())?;
//! let restored = Graph::from_snapshot(Snapshot::from_bytes(&a.snapshot().to_bytes()?)?);
//! assert_eq!(restored.edges().count(), 1);
//! assert_eq!(restored.node("test:17").unwrap().property("passed"), Some(&true.into()));
//! # Ok::<(), zergraph::Error>(())
//! ```

mod snapshot;
mod state;
mod sync;

pub use serde_json::Value;
pub use snapshot::Snapshot;
use state::{Entity, Property, Register, Stamp, State};
use std::{
    collections::{BTreeMap, BTreeSet},
    error, fmt,
};
pub use sync::{Checkpoint, Delta, MergeChanges};
use uuid::Uuid;

/// Identity of a directed labeled edge. Strings may contain any UTF-8 text.
/// Distinct labels allow different relationships between the same endpoints.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EdgeKey {
    pub source: String,
    pub label: String,
    pub target: String,
}
impl EdgeKey {
    pub fn new(
        source: impl Into<String>,
        label: impl Into<String>,
        target: impl Into<String>,
    ) -> Self {
        Self {
            source: source.into(),
            label: label.into(),
            target: target.into(),
        }
    }
}

/// Errors at graph mutation, merge, or snapshot boundaries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    MissingNode(String),
    MissingEdge(EdgeKey),
    ClockExhausted,
    ConflictingStamp,
    InvalidSnapshot(String),
    UnsupportedVersion(u32),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode(id) => write!(f, "node {id:?} is not visible"),
            Self::MissingEdge(key) => write!(f, "edge {key:?} is not visible"),
            Self::ClockExhausted => f.write_str("logical clock is exhausted"),
            Self::ConflictingStamp => f.write_str("same writer stamp has conflicting values"),
            Self::InvalidSnapshot(reason) => write!(f, "invalid snapshot: {reason}"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported snapshot version {version}")
            }
        }
    }
}
impl error::Error for Error {}

/// An immutable node view. Mutation goes through its owning graph.
#[derive(Debug, Clone, Copy)]
pub struct Node<'a> {
    id: &'a str,
    entity: &'a Entity,
}
impl<'a> Node<'a> {
    pub fn id(&self) -> &'a str {
        self.id
    }
    pub fn property(&self, key: &str) -> Option<&'a Value> {
        self.entity.property(key)
    }
    /// Live properties in lexical key order. JSON null is a live value.
    pub fn properties(&self) -> impl Iterator<Item = (&'a str, &'a Value)> {
        self.entity.properties()
    }
}

/// An immutable, visible edge view.
#[derive(Debug, Clone, Copy)]
pub struct Edge<'a> {
    key: &'a EdgeKey,
    entity: &'a Entity,
}
impl<'a> Edge<'a> {
    pub fn key(&self) -> &'a EdgeKey {
        self.key
    }
    pub fn property(&self, key: &str) -> Option<&'a Value> {
        self.entity.property(key)
    }
    pub fn properties(&self) -> impl Iterator<Item = (&'a str, &'a Value)> {
        self.entity.properties()
    }
}

/// One mutable replica. The caller owns transport, storage and synchronization.
/// Reads are deterministic; there are no background tasks or storage/network I/O.
#[derive(Debug)]
pub struct Graph {
    writer: Uuid,
    clock: u64,
    state: State,
    incoming: BTreeMap<String, BTreeSet<EdgeKey>>,
}

impl Default for Graph {
    /// Like `new`, this creates a fresh writer identity.
    fn default() -> Self {
        Self::new()
    }
}

impl Graph {
    /// Create an empty graph with a fresh random writer identity.
    pub fn new() -> Self {
        Self::from_snapshot(Snapshot::empty())
    }
    /// Start an independent writer at the current state.
    pub fn fork(&self) -> Self {
        Self {
            writer: Uuid::new_v4(),
            clock: self.clock,
            state: self.state.clone(),
            incoming: self.incoming.clone(),
        }
    }
    /// Restore complete state with a fresh writer identity, never the old writer.
    pub fn from_snapshot(snapshot: Snapshot) -> Self {
        let mut incoming = BTreeMap::new();
        for key in snapshot.state.edges.keys() {
            index_edge(&mut incoming, key);
        }
        Self {
            writer: Uuid::new_v4(),
            clock: snapshot.state.max_clock(),
            state: snapshot.state,
            incoming,
        }
    }
    /// Capture complete mergeable state, including deletions and hidden edges.
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            state: self.state.clone(),
        }
    }
    /// Merge complete peer state. Returns whether stored state changed, including
    /// hidden metadata. A conflicting stamp returns an error without mutation.
    pub fn merge(&mut self, other: &Snapshot) -> Result<bool, Error> {
        self.merge_state(&other.state)
    }
    fn merge_state(&mut self, other: &State) -> Result<bool, Error> {
        let plan = self.state.prepare(other)?;
        self.clock = self.clock.max(plan.clock);
        let incoming = &mut self.incoming;
        Ok(self.state.merge(plan, |key| index_edge(incoming, key)))
    }
    fn tick(&mut self) -> Result<Stamp, Error> {
        let counter = self.clock.checked_add(1).ok_or(Error::ClockExhausted)?;
        self.clock = counter;
        Ok(Stamp {
            counter,
            writer: self.writer,
        })
    }
    /// Add or revive a node. Already-live nodes are unchanged, including properties.
    pub fn add_node(&mut self, id: impl Into<String>) -> Result<bool, Error> {
        let id = id.into();
        if self.node(&id).is_some() {
            return Ok(false);
        }
        let stamp = self.tick()?;
        match self.state.nodes.get_mut(&id) {
            Some(entity) => entity.live = Register { stamp, value: true },
            None => {
                self.state.nodes.insert(id, Entity::new(stamp));
            }
        }
        Ok(true)
    }
    /// Hide a live node and its incident edges. Unknown/deleted IDs are no-ops.
    /// Same-ID revival preserves prior properties and live incident relationships.
    pub fn remove_node(&mut self, id: &str) -> Result<bool, Error> {
        if self.node(id).is_none() {
            return Ok(false);
        }
        let stamp = self.tick()?;
        self.state.nodes.get_mut(id).unwrap().live = Register {
            stamp,
            value: false,
        };
        Ok(true)
    }
    /// Add or revive a labeled edge. Both endpoints must be visible.
    pub fn add_edge(&mut self, key: EdgeKey) -> Result<bool, Error> {
        self.require_node(&key.source)?;
        self.require_node(&key.target)?;
        if self.edge(&key).is_some() {
            return Ok(false);
        }
        let stamp = self.tick()?;
        match self.state.edges.get_mut(&key) {
            Some(entity) => entity.live = Register { stamp, value: true },
            None => {
                index_edge(&mut self.incoming, &key);
                self.state.edges.insert(key, Entity::new(stamp));
            }
        }
        Ok(true)
    }
    /// Remove an edge's own membership, even when an endpoint currently hides it.
    /// This keeps it removed when that endpoint is revived.
    pub fn remove_edge(&mut self, key: &EdgeKey) -> Result<bool, Error> {
        if !self.state.edges.get(key).is_some_and(|e| e.live.value) {
            return Ok(false);
        }
        let stamp = self.tick()?;
        self.state.edges.get_mut(key).unwrap().live = Register {
            stamp,
            value: false,
        };
        Ok(true)
    }
    pub fn node(&self, id: &str) -> Option<Node<'_>> {
        let (id, entity) = self.state.nodes.get_key_value(id)?;
        entity.live.value.then_some(Node { id, entity })
    }
    pub fn edge(&self, key: &EdgeKey) -> Option<Edge<'_>> {
        let (key, entity) = self.state.edges.get_key_value(key)?;
        self.visible_edge(key, entity)
            .then_some(Edge { key, entity })
    }
    fn visible_edge(&self, key: &EdgeKey, entity: &Entity) -> bool {
        entity.live.value && self.node(&key.source).is_some() && self.node(&key.target).is_some()
    }
    /// Visible nodes in lexical ID order.
    pub fn nodes(&self) -> impl Iterator<Item = Node<'_>> {
        self.state
            .nodes
            .iter()
            .filter(|(_, e)| e.live.value)
            .map(|(id, entity)| Node { id, entity })
    }
    /// Visible edges in `(source, label, target)` order.
    pub fn edges(&self) -> impl Iterator<Item = Edge<'_>> {
        self.state
            .edges
            .iter()
            .filter(|(k, e)| self.visible_edge(k, e))
            .map(|(key, entity)| Edge { key, entity })
    }
    /// Visible outgoing edges, seeking directly to this source's ordered range.
    pub fn outgoing<'a>(&'a self, node: &'a str) -> impl Iterator<Item = Edge<'a>> {
        self.state
            .edges
            .range(EdgeKey::new(node, "", "")..)
            .take_while(move |(key, _)| key.source == node)
            .filter(|(key, entity)| self.visible_edge(key, entity))
            .map(|(key, entity)| Edge { key, entity })
    }
    /// Visible incoming edges, using a derived index of retained edge identities.
    pub fn incoming<'a>(&'a self, node: &'a str) -> impl Iterator<Item = Edge<'a>> {
        self.incoming
            .get(node)
            .into_iter()
            .flatten()
            .filter_map(|key| self.edge(key))
    }
    fn require_node(&self, id: &str) -> Result<(), Error> {
        self.node(id)
            .map(|_| ())
            .ok_or_else(|| Error::MissingNode(id.into()))
    }
    fn require_edge(&self, key: &EdgeKey) -> Result<(), Error> {
        self.edge(key)
            .map(|_| ())
            .ok_or_else(|| Error::MissingEdge(key.clone()))
    }
    /// Write a property on a visible node. Every call records a fresh write,
    /// including a repeated value, which can win a concurrent write.
    pub fn set_node_property(
        &mut self,
        id: &str,
        key: impl Into<String>,
        value: impl Into<Value>,
    ) -> Result<bool, Error> {
        self.require_node(id)?;
        let stamp = self.tick()?;
        self.state.nodes.get_mut(id).unwrap().properties.insert(
            key.into(),
            Register {
                stamp,
                value: Property::value(value.into()),
            },
        );
        Ok(true)
    }
    /// Remove a visible node's property, preserving its deletion stamp.
    pub fn remove_node_property(&mut self, id: &str, key: &str) -> Result<bool, Error> {
        self.require_node(id)?;
        if self.state.nodes[id].property(key).is_none() {
            return Ok(false);
        }
        let stamp = self.tick()?;
        self.state.nodes.get_mut(id).unwrap().properties.insert(
            key.into(),
            Register {
                stamp,
                value: Property::Removed,
            },
        );
        Ok(true)
    }
    /// Write a property on a visible edge, recording a fresh write.
    pub fn set_edge_property(
        &mut self,
        edge: &EdgeKey,
        key: impl Into<String>,
        value: impl Into<Value>,
    ) -> Result<bool, Error> {
        self.require_edge(edge)?;
        let stamp = self.tick()?;
        self.state.edges.get_mut(edge).unwrap().properties.insert(
            key.into(),
            Register {
                stamp,
                value: Property::value(value.into()),
            },
        );
        Ok(true)
    }
    pub fn remove_edge_property(&mut self, edge: &EdgeKey, key: &str) -> Result<bool, Error> {
        self.require_edge(edge)?;
        if self.state.edges[edge].property(key).is_none() {
            return Ok(false);
        }
        let stamp = self.tick()?;
        self.state.edges.get_mut(edge).unwrap().properties.insert(
            key.into(),
            Register {
                stamp,
                value: Property::Removed,
            },
        );
        Ok(true)
    }
}

// Index hidden/deleted identities too: deletion and revival only affect visibility.
fn index_edge(index: &mut BTreeMap<String, BTreeSet<EdgeKey>>, key: &EdgeKey) {
    index
        .entry(key.target.clone())
        .or_default()
        .insert(key.clone());
}
