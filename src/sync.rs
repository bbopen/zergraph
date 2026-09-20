use crate::{
    index_edge,
    state::{Entity, Stamp, State},
    EdgeKey, Error, Graph, Snapshot,
};
use std::collections::{BTreeMap, BTreeSet};

/// Exact register stamps without property values. Keep one per peer and advance
/// it only after that peer acknowledges successful application. Default means no known state.
#[derive(Debug, Clone, Default)]
pub struct Checkpoint {
    nodes: BTreeMap<String, Known>,
    edges: BTreeMap<EdgeKey, Known>,
}

#[derive(Debug, Clone)]
struct Known {
    live: Stamp,
    properties: Vec<(String, Stamp)>,
}

/// Mergeable partial state, including deletion stamps and edge endpoint records.
/// A delta is not a backup. Delivery of its acknowledged baseline is still needed
/// for full convergence. Duplicates and out-of-order delivery are safe.
#[derive(Debug, Clone, PartialEq)]
pub struct Delta(Snapshot);

impl Delta {
    /// Encode version 1 JSON with `kind: "delta"`, distinct from full snapshots.
    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        self.0.encode(Some("delta"))
    }
    /// Decode and validate a delta before applying it to any graph.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        Snapshot::decode(bytes, Some("delta")).map(Self)
    }
    pub fn is_empty(&self) -> bool {
        self.0.state.nodes.is_empty() && self.0.state.edges.is_empty()
    }
}

/// Stored records changed by a merge, plus edges affected by endpoint membership.
/// IDs are sorted and unique. Reread each ID; absent views are hidden or deleted.
/// A newer stamp may be reported even when the visible value stays the same.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MergeChanges {
    pub nodes: Vec<String>,
    pub edges: Vec<EdgeKey>,
}

impl MergeChanges {
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty() && self.edges.is_empty()
    }
}

impl Graph {
    /// Capture a complete checkpoint only if retained registers fit the limit.
    /// Counts node/edge membership and properties, including deletion markers.
    /// Returns `None` before copying if the limit is exceeded; never clips state.
    /// This limits register count, not bytes. IDs and property names vary in size.
    pub fn checkpoint_with_limit(&self, max_registers: usize) -> Option<Checkpoint> {
        self.state
            .nodes
            .values()
            .chain(self.state.edges.values())
            .try_fold(max_registers, |remaining, entity| {
                remaining
                    .checked_sub(1)?
                    .checked_sub(entity.properties.len())
            })?;
        Some(self.checkpoint())
    }
    /// Capture register knowledge before constructing the outgoing delta. Retain
    /// it only after that delta is acknowledged; later edits may not have been sent.
    pub fn checkpoint(&self) -> Checkpoint {
        fn capture<K: Ord + Clone>(map: &BTreeMap<K, Entity>) -> BTreeMap<K, Known> {
            map.iter()
                .map(|(key, entity)| {
                    (
                        key.clone(),
                        Known {
                            live: entity.live.stamp,
                            properties: entity
                                .properties
                                .iter()
                                .map(|(k, r)| (k.clone(), r.stamp))
                                .collect(),
                        },
                    )
                })
                .collect()
        }
        Checkpoint {
            nodes: capture(&self.state.nodes),
            edges: capture(&self.state.edges),
        }
    }
    /// Scan retained state and copy registers newer than the peer's checkpoint.
    /// Retry from the last acknowledged checkpoint after a dropped message. Reset
    /// the checkpoint for a peer that lost state. This method retains no history.
    pub fn delta_since(&self, known: &Checkpoint) -> Delta {
        let mut state = State {
            nodes: difference(&self.state.nodes, &known.nodes),
            edges: difference(&self.state.edges, &known.edges),
        };
        for key in state.edges.keys() {
            for id in [&key.source, &key.target] {
                state.nodes.entry(id.clone()).or_insert_with(|| Entity {
                    live: self.state.nodes[id].live.clone(),
                    properties: BTreeMap::new(),
                });
            }
        }
        Delta(Snapshot { state })
    }
    /// Merge partial state atomically. Returns whether stored state changed.
    pub fn apply_delta(&mut self, delta: &Delta) -> Result<bool, Error> {
        self.merge_state(&delta.0.state)
    }
    /// Merge partial state and report records to reread, including affected edges.
    pub fn apply_delta_with_changes(&mut self, delta: &Delta) -> Result<MergeChanges, Error> {
        self.merge_reporting(&delta.0.state)
    }
    /// Merge full state and report records to reread, including affected edges.
    pub fn merge_with_changes(&mut self, snapshot: &Snapshot) -> Result<MergeChanges, Error> {
        self.merge_reporting(&snapshot.state)
    }
    fn merge_reporting(&mut self, state: &State) -> Result<MergeChanges, Error> {
        let plan = self.state.prepare(state)?;
        let mut edges: BTreeSet<EdgeKey> = plan.edges.iter().map(|(k, _)| (*k).clone()).collect();
        for (id, entity) in &plan.nodes {
            if self.state.nodes.get(*id).is_some_and(|old| {
                old.live.value != entity.live.value && entity.live.stamp > old.live.stamp
            }) {
                let start = EdgeKey::new((*id).clone(), "", "");
                edges.extend(
                    self.state
                        .edges
                        .range(start..)
                        .take_while(|(key, _)| &key.source == *id)
                        .map(|(key, _)| key.clone()),
                );
                if let Some(incoming) = self.incoming.get(*id) {
                    edges.extend(incoming.iter().cloned());
                }
            }
        }
        let changes = MergeChanges {
            nodes: plan.nodes.iter().map(|(id, _)| (*id).clone()).collect(),
            edges: edges.into_iter().collect(),
        };
        self.clock = self.clock.max(plan.clock);
        let incoming = &mut self.incoming;
        self.state.merge(plan, |key| index_edge(incoming, key));
        Ok(changes)
    }
}

fn difference<K: Ord + Clone>(
    state: &BTreeMap<K, Entity>,
    known: &BTreeMap<K, Known>,
) -> BTreeMap<K, Entity> {
    let mut aligned = known.iter();
    state
        .iter()
        .filter_map(|(key, entity)| {
            let old = aligned
                .next()
                .filter(|(k, _)| *k == key)
                .map(|(_, entity)| entity)
                .or_else(|| known.get(key));
            let properties: BTreeMap<_, _> = entity
                .properties
                .iter()
                .filter(|(key, register)| {
                    old.and_then(|e| {
                        e.properties
                            .binary_search_by(|(k, _)| k.cmp(key))
                            .ok()
                            .map(|i| &e.properties[i].1)
                    })
                    .is_none_or(|stamp| register.stamp > *stamp)
                })
                .map(|(key, register)| (key.clone(), register.clone()))
                .collect();
            if !properties.is_empty() || old.is_none_or(|e| entity.live.stamp > e.live) {
                Some((
                    key.clone(),
                    Entity {
                        live: entity.live.clone(),
                        properties,
                    },
                ))
            } else {
                None
            }
        })
        .collect()
}
