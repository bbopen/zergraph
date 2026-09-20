use crate::{
    state::{Entity, Property, State},
    EdgeKey, Error,
};
use serde::{Deserialize, Serialize};

/// Immutable complete state, safe to clone, persist, and exchange repeatedly.
/// The encoding contains causal metadata and may include deleted values.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub(crate) state: State,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    version: u32,
    nodes: Vec<(String, Entity)>,
    edges: Vec<(EdgeKey, Entity)>,
}

impl Snapshot {
    pub(crate) fn empty() -> Self {
        Self {
            state: State::default(),
        }
    }
    /// Encode version 1 JSON in stable order, including tombstones and hidden state.
    /// Transport framing, file replacement and durability are caller concerns.
    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut wire = Wire {
            version: 1,
            nodes: self
                .state
                .nodes
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            edges: self
                .state
                .edges
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        };
        // Downstream crates can enable serde_json/preserve_order through feature
        // unification. Keep nested user objects ordered in that configuration too.
        for entity in wire
            .nodes
            .iter_mut()
            .map(|(_, e)| e)
            .chain(wire.edges.iter_mut().map(|(_, e)| e))
        {
            for register in entity.properties.values_mut() {
                if let Property::Value(value) = &mut register.value {
                    value.sort_all_objects();
                }
            }
        }
        serde_json::to_vec(&wire).map_err(|e| Error::InvalidSnapshot(e.to_string()))
    }
    /// Decode complete state. Unknown versions, duplicate identities, zero stamps,
    /// and edges without endpoint records are rejected. Deleted endpoints are valid.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let wire: Wire =
            serde_json::from_slice(bytes).map_err(|e| Error::InvalidSnapshot(e.to_string()))?;
        if wire.version != 1 {
            return Err(Error::UnsupportedVersion(wire.version));
        }
        let mut state = State::default();
        for (id, entity) in wire.nodes {
            if state.nodes.insert(id, entity).is_some() {
                return Err(Error::InvalidSnapshot("duplicate node ID".into()));
            }
        }
        for (key, entity) in wire.edges {
            if !state.nodes.contains_key(&key.source) || !state.nodes.contains_key(&key.target) {
                return Err(Error::InvalidSnapshot(
                    "edge endpoint record is missing".into(),
                ));
            }
            if state.edges.insert(key, entity).is_some() {
                return Err(Error::InvalidSnapshot("duplicate edge key".into()));
            }
        }
        for stamp in state
            .nodes
            .values()
            .chain(state.edges.values())
            .flat_map(Entity::stamps)
        {
            if stamp.counter == 0 || stamp.writer.is_nil() {
                return Err(Error::InvalidSnapshot("invalid writer stamp".into()));
            }
        }
        Ok(Self { state })
    }
}
