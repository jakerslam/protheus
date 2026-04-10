// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/ops (authoritative hot-path memory scaffolding)

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};

const ARENA_CAPACITY_BYTES: usize = 256 * 1024;
const SLAB_BUCKET_CAPS: [usize; 6] = [64, 256, 1024, 4096, 16384, 65536];
const SLAB_MAX_BUCKET_DEPTH: usize = 64;

static ARENA_ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static ARENA_BYTES_REQUESTED: AtomicU64 = AtomicU64::new(0);
static ARENA_RESETS: AtomicU64 = AtomicU64::new(0);
static ARENA_FALLBACKS: AtomicU64 = AtomicU64::new(0);
static SLAB_CHECKOUTS: AtomicU64 = AtomicU64::new(0);
static SLAB_REUSES: AtomicU64 = AtomicU64::new(0);
static SLAB_FALLBACKS: AtomicU64 = AtomicU64::new(0);
static SLAB_RETURNS: AtomicU64 = AtomicU64::new(0);
static HOT_PATH_MARKS: AtomicU64 = AtomicU64::new(0);
static HOT_PATH_ARENA_BYTES: AtomicU64 = AtomicU64::new(0);
static HOT_PATH_SLAB_BYTES: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HotAllocatorSnapshot {
    pub arena_allocations: u64,
    pub arena_bytes_requested: u64,
    pub arena_resets: u64,
    pub arena_fallbacks: u64,
    pub slab_checkouts: u64,
    pub slab_reuses: u64,
    pub slab_fallbacks: u64,
    pub slab_returns: u64,
    pub slab_cached_buffers: u64,
    pub hot_path_marks: u64,
    pub hot_path_arena_bytes: u64,
    pub hot_path_slab_bytes: u64,
}

impl HotAllocatorSnapshot {
    fn arena_fallback_rate_pct(self) -> f64 {
        rate_pct(self.arena_fallbacks, self.arena_allocations)
    }

    fn slab_hit_rate_pct(self) -> f64 {
        rate_pct(self.slab_reuses, self.slab_checkouts)
    }
}

fn rate_pct(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        return 0.0;
    }
    ((numerator as f64 / denominator as f64) * 10000.0).round() / 100.0
}

struct BumpArena {
    storage: Vec<u8>,
    cursor: usize,
}

impl BumpArena {
    fn new(capacity: usize) -> Self {
        Self {
            storage: vec![0u8; capacity.max(4096)],
            cursor: 0,
        }
    }

    fn capacity(&self) -> usize {
        self.storage.len()
    }

    fn remaining(&self) -> usize {
        self.capacity().saturating_sub(self.cursor)
    }

    fn reset(&mut self) {
        self.cursor = 0;
    }

    fn alloc(&mut self, len: usize) -> Option<&mut [u8]> {
        if len == 0 {
            return Some(&mut self.storage[0..0]);
        }
        if len > self.remaining() {
            return None;
        }
        let start = self.cursor;
        self.cursor += len;
        Some(&mut self.storage[start..start + len])
    }
}

struct ByteSlab {
    buckets: Vec<Vec<Vec<u8>>>,
}

impl ByteSlab {
    fn new() -> Self {
        Self {
            buckets: (0..SLAB_BUCKET_CAPS.len()).map(|_| Vec::new()).collect(),
        }
    }

    fn bucket_for(size: usize) -> Option<usize> {
        SLAB_BUCKET_CAPS.iter().position(|cap| size <= *cap)
    }

    fn checkout(&mut self, minimum_capacity: usize) -> Vec<u8> {
        SLAB_CHECKOUTS.fetch_add(1, Ordering::Relaxed);
        let requested = minimum_capacity.max(1);
        match Self::bucket_for(requested) {
            Some(bucket_idx) => {
                if let Some(mut reused) = self.buckets[bucket_idx].pop() {
                    reused.clear();
                    SLAB_REUSES.fetch_add(1, Ordering::Relaxed);
                    reused
                } else {
                    Vec::with_capacity(SLAB_BUCKET_CAPS[bucket_idx])
                }
            }
            None => {
                SLAB_FALLBACKS.fetch_add(1, Ordering::Relaxed);
                Vec::with_capacity(requested)
            }
        }
    }

    fn checkin(&mut self, mut buffer: Vec<u8>) {
        let capacity = buffer.capacity();
        if let Some(bucket_idx) = Self::bucket_for(capacity) {
            if self.buckets[bucket_idx].len() < SLAB_MAX_BUCKET_DEPTH {
                buffer.clear();
                self.buckets[bucket_idx].push(buffer);
                SLAB_RETURNS.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    fn cached_buffers(&self) -> usize {
        self.buckets.iter().map(Vec::len).sum()
    }
}

thread_local! {
    static TLS_ARENA: RefCell<BumpArena> = RefCell::new(BumpArena::new(ARENA_CAPACITY_BYTES));
    static TLS_SLAB: RefCell<ByteSlab> = RefCell::new(ByteSlab::new());
}

pub fn mark_hot_path_batch(arena_bytes: usize, slab_bytes: usize) {
    HOT_PATH_MARKS.fetch_add(1, Ordering::Relaxed);
    HOT_PATH_ARENA_BYTES.fetch_add(arena_bytes as u64, Ordering::Relaxed);
    HOT_PATH_SLAB_BYTES.fetch_add(slab_bytes as u64, Ordering::Relaxed);
}

pub fn with_arena_bytes<R>(len: usize, f: impl FnOnce(&mut [u8]) -> R) -> R {
    ARENA_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
    ARENA_BYTES_REQUESTED.fetch_add(len as u64, Ordering::Relaxed);
    if len == 0 {
        let mut empty = [];
        return f(&mut empty);
    }
    TLS_ARENA.with(|cell| {
        let mut arena = cell.borrow_mut();
        if len > arena.capacity() {
            ARENA_FALLBACKS.fetch_add(1, Ordering::Relaxed);
            let mut fallback = vec![0u8; len];
            return f(fallback.as_mut_slice());
        }
        if len > arena.remaining() {
            arena.reset();
            ARENA_RESETS.fetch_add(1, Ordering::Relaxed);
        }
        if let Some(slice) = arena.alloc(len) {
            return f(slice);
        }
        ARENA_FALLBACKS.fetch_add(1, Ordering::Relaxed);
        let mut fallback = vec![0u8; len];
        f(fallback.as_mut_slice())
    })
}

pub fn with_slab_buffer<R>(minimum_capacity: usize, f: impl FnOnce(&mut Vec<u8>) -> R) -> R {
    TLS_SLAB.with(|cell| {
        let mut slab = cell.borrow_mut();
        let mut buffer = slab.checkout(minimum_capacity);
        drop(slab);
        let out = f(&mut buffer);
        let mut slab = cell.borrow_mut();
        slab.checkin(buffer);
        out
    })
}

fn hash_quoted_json(raw: &str, hasher: &mut Sha256) {
    with_slab_buffer(raw.len().saturating_add(8), |buffer| {
        buffer.clear();
        if serde_json::to_writer(&mut *buffer, raw).is_ok() {
            hasher.update(buffer.as_slice());
        } else {
            hasher.update(b"\"\"");
        }
    });
}

fn stable_hash_update(value: &Value, hasher: &mut Sha256) {
    match value {
        Value::Null => hasher.update(b"null"),
        Value::Bool(true) => hasher.update(b"true"),
        Value::Bool(false) => hasher.update(b"false"),
        Value::Number(number) => hasher.update(number.to_string().as_bytes()),
        Value::String(raw) => hash_quoted_json(raw, hasher),
        Value::Array(rows) => {
            hasher.update(b"[");
            for (idx, row) in rows.iter().enumerate() {
                if idx > 0 {
                    hasher.update(b",");
                }
                stable_hash_update(row, hasher);
            }
            hasher.update(b"]");
        }
        Value::Object(map) => {
            let mut keys = map.keys().map(|key| key.as_str()).collect::<Vec<_>>();
            keys.sort_unstable();
            hasher.update(b"{");
            for (idx, key) in keys.iter().enumerate() {
                if idx > 0 {
                    hasher.update(b",");
                }
                hash_quoted_json(key, hasher);
                hasher.update(b":");
                stable_hash_update(map.get(*key).unwrap_or(&Value::Null), hasher);
            }
            hasher.update(b"}");
        }
    }
}

pub fn deterministic_hash(value: &Value) -> String {
    let mut hasher = Sha256::new();
    stable_hash_update(value, &mut hasher);
    hex::encode(hasher.finalize())
}

pub fn snapshot() -> HotAllocatorSnapshot {
    let slab_cached_buffers = TLS_SLAB.with(|cell| cell.borrow().cached_buffers() as u64);
    HotAllocatorSnapshot {
        arena_allocations: ARENA_ALLOCATIONS.load(Ordering::Relaxed),
        arena_bytes_requested: ARENA_BYTES_REQUESTED.load(Ordering::Relaxed),
        arena_resets: ARENA_RESETS.load(Ordering::Relaxed),
        arena_fallbacks: ARENA_FALLBACKS.load(Ordering::Relaxed),
        slab_checkouts: SLAB_CHECKOUTS.load(Ordering::Relaxed),
        slab_reuses: SLAB_REUSES.load(Ordering::Relaxed),
        slab_fallbacks: SLAB_FALLBACKS.load(Ordering::Relaxed),
        slab_returns: SLAB_RETURNS.load(Ordering::Relaxed),
        slab_cached_buffers,
        hot_path_marks: HOT_PATH_MARKS.load(Ordering::Relaxed),
        hot_path_arena_bytes: HOT_PATH_ARENA_BYTES.load(Ordering::Relaxed),
        hot_path_slab_bytes: HOT_PATH_SLAB_BYTES.load(Ordering::Relaxed),
    }
}

pub fn snapshot_json() -> Value {
    let snapshot = snapshot();
    json!({
        "arena": {
            "allocations": snapshot.arena_allocations,
            "bytes_requested": snapshot.arena_bytes_requested,
            "resets": snapshot.arena_resets,
            "fallbacks": snapshot.arena_fallbacks,
            "fallback_rate_pct": snapshot.arena_fallback_rate_pct()
        },
        "slab": {
            "checkouts": snapshot.slab_checkouts,
            "reuses": snapshot.slab_reuses,
            "fallbacks": snapshot.slab_fallbacks,
            "returns": snapshot.slab_returns,
            "cached_buffers": snapshot.slab_cached_buffers,
            "hit_rate_pct": snapshot.slab_hit_rate_pct()
        },
        "hot_path_batches": {
            "marks": snapshot.hot_path_marks,
            "arena_bytes": snapshot.hot_path_arena_bytes,
            "slab_bytes": snapshot.hot_path_slab_bytes
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_hash_is_key_order_invariant() {
        let a = json!({"z":2,"a":{"k":"v","n":1},"arr":[3,2,1]});
        let b = json!({"arr":[3,2,1],"a":{"n":1,"k":"v"},"z":2});
        assert_eq!(deterministic_hash(&a), deterministic_hash(&b));
    }

    #[test]
    fn slab_pool_reuses_returned_buffers() {
        let before = snapshot();
        with_slab_buffer(48, |buf| {
            buf.extend_from_slice(b"alpha");
        });
        with_slab_buffer(48, |buf| {
            buf.extend_from_slice(b"beta");
        });
        let after = snapshot();
        assert!(after.slab_checkouts >= before.slab_checkouts + 2);
        assert!(after.slab_reuses >= before.slab_reuses + 1);
    }

    #[test]
    fn arena_allocates_and_can_fallback_for_large_requests() {
        let before = snapshot();
        with_arena_bytes(128, |slice| {
            slice[0] = 1;
        });
        with_arena_bytes(ARENA_CAPACITY_BYTES * 2, |slice| {
            slice[0] = 2;
        });
        let after = snapshot();
        assert!(after.arena_allocations >= before.arena_allocations + 2);
        assert!(after.arena_fallbacks >= before.arena_fallbacks + 1);
    }
}
