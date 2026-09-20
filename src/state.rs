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

impl Property {
    // Normalize once at every value ingress, so snapshots can borrow stored data.
    pub fn value(mut value: Value) -> Self {
        value.sort_all_objects();
        Self::Value(value)
    }
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
    fn check_merge(&self, other: &Self) -> Result<bool, Error> {
        self.live.check_merge(&other.live)?;
        let mut changed = other.live.stamp > self.live.stamp;
        for (key, reg) in &other.properties {
            if let Some(ours) = self.properties.get(key) {
                ours.check_merge(reg)?;
                changed |= reg.stamp > ours.stamp;
            } else {
                changed = true;
            }
        }
        Ok(changed)
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

// Borrow only changed incoming entities. Validate the complete merge before any
// mutation, then avoid a second traversal of unchanged records.
pub(crate) struct MergePlan<'a> {
    nodes: Vec<(&'a String, &'a Entity)>,
    edges: Vec<(&'a EdgeKey, &'a Entity)>,
    pub clock: u64,
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
    pub fn prepare<'a>(&self, other: &'a Self) -> Result<MergePlan<'a>, Error> {
        let mut clock = 0;
        let nodes = changes(&self.nodes, &other.nodes, &mut clock)?;
        let edges = changes(&self.edges, &other.edges, &mut clock)?;
        Ok(MergePlan {
            nodes,
            edges,
            clock,
        })
    }
    pub fn merge(&mut self, plan: MergePlan<'_>, on_edge: impl FnMut(&EdgeKey)) -> bool {
        merge_map(&mut self.nodes, plan.nodes, |_| {})
            | merge_map(&mut self.edges, plan.edges, on_edge)
    }
}

fn changes<'a, K: Ord>(
    ours: &BTreeMap<K, Entity>,
    theirs: &'a BTreeMap<K, Entity>,
    clock: &mut u64,
) -> Result<Vec<(&'a K, &'a Entity)>, Error> {
    let mut aligned = ours.iter();
    let mut changed = Vec::new();
    for (key, entity) in theirs {
        // Full snapshots commonly have aligned keys. Sparse or shifted keys fall
        // back to lookup, without scanning a much larger local graph.
        let existing = aligned
            .next()
            .filter(|(k, _)| *k == key)
            .map(|(_, v)| v)
            .or_else(|| ours.get(key));
        *clock = (*clock).max(entity.stamps().map(|s| s.counter).max().unwrap());
        if existing
            .map(|local| local.check_merge(entity))
            .transpose()?
            .unwrap_or(true)
        {
            changed.push((key, entity));
        }
    }
    Ok(changed)
}

fn merge_map<K: Ord + Clone>(
    ours: &mut BTreeMap<K, Entity>,
    theirs: Vec<(&K, &Entity)>,
    mut on_new: impl FnMut(&K),
) -> bool {
    let mut changed = false;
    for (key, entity) in theirs {
        match ours.get_mut(key) {
            Some(ours) => changed |= ours.merge(entity),
            None => {
                on_new(key);
                ours.insert(key.clone(), entity.clone());
                changed = true;
            }
        }
    }
    changed
}
