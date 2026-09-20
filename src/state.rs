use crate::{EdgeKey, Error, Value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Stamp {
    pub counter: u64,
    pub writer: Uuid,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Register<T> {
    pub stamp: Stamp,
    pub value: T,
}
impl<T: Clone + PartialEq> Register<T> {
    fn check_merge(&self, other: &Self) -> Result<(), Error> {
        if self.stamp == other.stamp && self.value != other.value {
            Err(Error::ConflictingStamp)
        } else {
            Ok(())
        }
    }
    fn merge(&mut self, other: &Self) -> bool {
        if other.stamp > self.stamp {
            *self = other.clone();
            true
        } else {
            false
        }
    }
}

// An explicit variant keeps JSON null distinct from a property tombstone.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub(crate) enum Property {
    Value(Value),
    Removed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Entity {
    pub live: Register<bool>,
    pub properties: BTreeMap<String, Register<Property>>,
}
impl Entity {
    pub fn new(stamp: Stamp) -> Self {
        Self {
            live: Register { stamp, value: true },
            properties: BTreeMap::new(),
        }
    }
    pub fn property(&self, key: &str) -> Option<&Value> {
        match &self.properties.get(key)?.value {
            Property::Value(v) => Some(v),
            Property::Removed => None,
        }
    }
    pub fn properties(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.properties
            .iter()
            .filter_map(|(key, r)| match &r.value {
                Property::Value(v) => Some((key.as_str(), v)),
                Property::Removed => None,
            })
    }
    fn check_merge(&self, other: &Self) -> Result<(), Error> {
        self.live.check_merge(&other.live)?;
        for (key, reg) in &other.properties {
            if let Some(ours) = self.properties.get(key) {
                ours.check_merge(reg)?;
            }
        }
        Ok(())
    }
    fn merge(&mut self, other: &Self) -> bool {
        let mut changed = self.live.merge(&other.live);
        for (key, reg) in &other.properties {
            match self.properties.get_mut(key) {
                Some(ours) => changed |= ours.merge(reg),
                None => {
                    self.properties.insert(key.clone(), reg.clone());
                    changed = true;
                }
            }
        }
        changed
    }
    pub fn stamps(&self) -> impl Iterator<Item = Stamp> + '_ {
        std::iter::once(self.live.stamp).chain(self.properties.values().map(|r| r.stamp))
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct State {
    pub nodes: BTreeMap<String, Entity>,
    pub edges: BTreeMap<EdgeKey, Entity>,
}
impl State {
    pub fn max_clock(&self) -> u64 {
        self.nodes
            .values()
            .chain(self.edges.values())
            .flat_map(Entity::stamps)
            .map(|s| s.counter)
            .max()
            .unwrap_or(0)
    }
    pub fn check_merge(&self, other: &Self) -> Result<(), Error> {
        for (id, entity) in &other.nodes {
            if let Some(ours) = self.nodes.get(id) {
                ours.check_merge(entity)?;
            }
        }
        for (key, entity) in &other.edges {
            if let Some(ours) = self.edges.get(key) {
                ours.check_merge(entity)?;
            }
        }
        Ok(())
    }
    pub fn merge(&mut self, other: &Self) -> bool {
        merge_map(&mut self.nodes, &other.nodes) | merge_map(&mut self.edges, &other.edges)
    }
}
fn merge_map<K: Ord + Clone>(ours: &mut BTreeMap<K, Entity>, theirs: &BTreeMap<K, Entity>) -> bool {
    let mut changed = false;
    for (key, entity) in theirs {
        match ours.get_mut(key) {
            Some(ours) => changed |= ours.merge(entity),
            None => {
                ours.insert(key.clone(), entity.clone());
                changed = true;
            }
        }
    }
    changed
}
