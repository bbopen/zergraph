//! Module for CRDT types used in the graph.

/// Last-Write-Wins Set
#[derive(Debug, Clone)]
pub struct LwwSet<T> {
    // Underlying set data structure
    pub items: Vec<T>,
}

impl<T> LwwSet<T> {
    /// Creates a new LWWSet.
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }
    // TODO: Implement insertion, removal and merge logic
}

/// Last-Write-Wins Map
#[derive(Debug, Clone)]
pub struct LwwMap<K, V> {
    // Underlying map data structure
    pub entries: Vec<(K, V)>,
}

impl<K, V> LwwMap<K, V> {
    /// Creates a new LWWMap.
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }
    // TODO: Implement insertion, removal and merge logic
}

/// Observed-Remove Set (OR-Set)
#[derive(Debug, Clone)]
pub struct OrSet<T> {
    // Underlying set data structure
    pub items: Vec<T>,
}

impl<T> OrSet<T> {
    /// Creates a new ORSet.
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }
    // TODO: Implement observed-remove semantics
}

/// Grow-only Counter (G-Counter)
#[derive(Debug, Clone)]
pub struct GCounter {
    // Underlying counter
    pub count: u64,
}

impl GCounter {
    /// Creates a new GCounter.
    pub fn new() -> Self {
        Self { count: 0 }
    }
    
    /// Increments the counter by 1
    pub fn inc(&mut self) {
        self.count += 1;
    }
    
    // TODO: Implement merge logic for distributed counters
} 