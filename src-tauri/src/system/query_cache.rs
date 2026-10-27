// OpenRevo - Generic TTL Query Cache
// Shared building blocks for cached hardware/system queries.
//
// Several background loops (1.5 s telemetry heartbeat, 500 ms idle watcher, 800 ms
// process scanner) re-run expensive OS queries that only change on rare events:
// full display-mode enumeration (~22 ms), `schtasks` process spawns, registry scans.
// Wrapping those queries in one of these caches keeps the "read" API unchanged while
// turning a repeated probe into a memory hit.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::hash::Hash;
use std::time::{Duration, Instant};

/// Single-slot cache holding one value until `ttl` elapses or it is invalidated.
///
/// The lock is never held while `compute` runs: a slow probe must not block other
/// callers, and holding it across a compute that re-enters this cache would deadlock.
pub struct ExpiringCache<V> {
    ttl: Duration,
    slot: RwLock<Option<(Instant, V)>>,
}

impl<V: Clone> ExpiringCache<V> {
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            slot: RwLock::new(None),
        }
    }

    /// Return the cached value, or compute and store it.
    pub fn get_or_compute<F>(&self, compute: F) -> V
    where
        F: FnOnce() -> V,
    {
        if let Some((stamped, value)) = self.slot.read().as_ref() {
            if stamped.elapsed() < self.ttl {
                return value.clone();
            }
        }

        let value = compute();
        *self.slot.write() = Some((Instant::now(), value.clone()));
        value
    }

    /// Cached value if it is still fresh.
    pub fn get(&self) -> Option<V> {
        self.slot
            .read()
            .as_ref()
            .filter(|(stamped, _)| stamped.elapsed() < self.ttl)
            .map(|(_, value)| value.clone())
    }

    /// Drop the cached value so the next call recomputes.
    pub fn invalidate(&self) {
        *self.slot.write() = None;
    }

    /// Store a known value, refreshing the timestamp (write-through after a mutation).
    pub fn put(&self, value: V) {
        *self.slot.write() = Some((Instant::now(), value));
    }
}

/// Per-key variant of [`ExpiringCache`], for queries parameterised by device name,
/// hardware id, profile path, etc.
pub struct KeyedExpiringCache<K, V> {
    ttl: Duration,
    map: RwLock<HashMap<K, (Instant, V)>>,
}

impl<K: Eq + Hash + Clone, V: Clone> KeyedExpiringCache<K, V> {
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            map: RwLock::new(HashMap::new()),
        }
    }

    /// Return the cached value for `key`, or compute and store it.
    pub fn get_or_compute<F>(&self, key: K, compute: F) -> V
    where
        F: FnOnce() -> V,
    {
        if let Some((stamped, value)) = self.map.read().get(&key) {
            if stamped.elapsed() < self.ttl {
                return value.clone();
            }
        }

        let value = compute();
        self.map
            .write()
            .insert(key, (Instant::now(), value.clone()));
        value
    }

    /// Cached value for `key` if it is still fresh.
    pub fn get(&self, key: &K) -> Option<V> {
        self.map
            .read()
            .get(key)
            .filter(|(stamped, _)| stamped.elapsed() < self.ttl)
            .map(|(_, value)| value.clone())
    }

    /// Drop every entry. Use when a system-wide event invalidates all keys
    /// (display topology change, config reload and so on).
    pub fn invalidate_all(&self) {
        self.map.write().clear();
    }

    /// Drop a single entry.
    pub fn invalidate(&self, key: &K) {
        self.map.write().remove(key);
    }

    /// Store a known value, refreshing the timestamp.
    pub fn put(&self, key: K, value: V) {
        self.map.write().insert(key, (Instant::now(), value));
    }
}
