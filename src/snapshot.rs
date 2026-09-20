use crate::{
    state::{Entity, Property, State},
    EdgeKey, Error,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Immutable complete state, safe to clone, persist, and exchange repeatedly.
/// The encoding contains causal metadata and may include deleted values.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub(crate) state: State,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire<N, E> {
    version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    kind: Option<String>,
    nodes: N,
    edges: E,
}
type Records<K> = Vec<(K, Entity)>;

// Preserve the version 1 array-of-pairs format without allocating a second graph.
struct Pairs<'a, K, V>(&'a BTreeMap<K, V>);
impl<K: Serialize, V: Serialize> Serialize for Pairs<'_, K, V> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.0.iter())
    }
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
        self.encode(None)
    }
    pub(crate) fn encode(&self, kind: Option<&str>) -> Result<Vec<u8>, Error> {
        let wire = Wire {
            version: 1,
            kind: kind.map(str::to_owned),
            nodes: Pairs(&self.state.nodes),
            edges: Pairs(&self.state.edges),
        };
        serde_json::to_vec(&wire).map_err(|e| Error::InvalidSnapshot(e.to_string()))
    }
    /// Decode complete state. Unknown versions, duplicate identities, zero stamps,
    /// and edges without endpoint records are rejected. Deleted endpoints are valid.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        Self::decode(bytes, None)
    }
    pub(crate) fn decode(bytes: &[u8], kind: Option<&str>) -> Result<Self, Error> {
        let wire: Wire<Records<String>, Records<EdgeKey>> =
            serde_json::from_slice(bytes).map_err(|e| Error::InvalidSnapshot(e.to_string()))?;
        if wire.version != 1 {
            return Err(Error::UnsupportedVersion(wire.version));
        }
        if wire.kind.as_deref() != kind {
            return Err(Error::InvalidSnapshot("wrong message kind".into()));
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
        for entity in state.nodes.values_mut().chain(state.edges.values_mut()) {
            for stamp in entity.stamps() {
                if stamp.counter == 0 || stamp.writer.is_nil() {
                    return Err(Error::InvalidSnapshot("invalid writer stamp".into()));
                }
            }
            for register in entity.properties.values_mut() {
                if let Property::Value(value) = &mut register.value {
                    value.sort_all_objects();
                }
            }
        }
        Ok(Self { state })
    }
}
