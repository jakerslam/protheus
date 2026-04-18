// SPDX-License-Identifier: Apache-2.0
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllocatorSnapshot {
    pub alloc_calls: usize,
    pub dealloc_calls: usize,
    pub alloc_failures: usize,
    pub bytes_requested: usize,
    pub bytes_released: usize,
    pub bytes_outstanding: usize,
}

pub struct Layer0CountingAllocator;

static ALLOC_CALLS: AtomicUsize = AtomicUsize::new(0);
static DEALLOC_CALLS: AtomicUsize = AtomicUsize::new(0);
static ALLOC_FAILURES: AtomicUsize = AtomicUsize::new(0);
static BYTES_REQUESTED: AtomicUsize = AtomicUsize::new(0);
static BYTES_RELEASED: AtomicUsize = AtomicUsize::new(0);
static BYTES_OUTSTANDING: AtomicUsize = AtomicUsize::new(0);

fn atomic_saturating_add(counter: &AtomicUsize, value: usize) {
    let _ = counter.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
        Some(current.saturating_add(value))
    });
}

fn atomic_saturating_sub(counter: &AtomicUsize, value: usize) {
    let _ = counter.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
        Some(current.saturating_sub(value))
    });
}

impl Layer0CountingAllocator {
    pub fn snapshot() -> AllocatorSnapshot {
        AllocatorSnapshot {
            alloc_calls: ALLOC_CALLS.load(Ordering::Relaxed),
            dealloc_calls: DEALLOC_CALLS.load(Ordering::Relaxed),
            alloc_failures: ALLOC_FAILURES.load(Ordering::Relaxed),
            bytes_requested: BYTES_REQUESTED.load(Ordering::Relaxed),
            bytes_released: BYTES_RELEASED.load(Ordering::Relaxed),
            bytes_outstanding: BYTES_OUTSTANDING.load(Ordering::Relaxed),
        }
    }
}

unsafe impl GlobalAlloc for Layer0CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        // SAFETY: delegated to the platform allocator with the same layout.
        let ptr = unsafe { System.alloc(layout) };
        atomic_saturating_add(&ALLOC_CALLS, 1);
        if ptr.is_null() {
            atomic_saturating_add(&ALLOC_FAILURES, 1);
        } else {
            atomic_saturating_add(&BYTES_REQUESTED, size);
            atomic_saturating_add(&BYTES_OUTSTANDING, size);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let size = layout.size();
        atomic_saturating_add(&DEALLOC_CALLS, 1);
        atomic_saturating_add(&BYTES_RELEASED, size);
        atomic_saturating_sub(&BYTES_OUTSTANDING, size);
        // SAFETY: delegated to the platform allocator with the same layout.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        // SAFETY: delegated to the platform allocator with the same layout.
        let ptr = unsafe { System.alloc_zeroed(layout) };
        atomic_saturating_add(&ALLOC_CALLS, 1);
        if ptr.is_null() {
            atomic_saturating_add(&ALLOC_FAILURES, 1);
        } else {
            atomic_saturating_add(&BYTES_REQUESTED, size);
            atomic_saturating_add(&BYTES_OUTSTANDING, size);
        }
        ptr
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let old_size = layout.size();
        // SAFETY: delegated to the platform allocator with the same layout.
        let new_ptr = unsafe { System.realloc(ptr, layout, new_size) };
        atomic_saturating_add(&ALLOC_CALLS, 1);
        if new_ptr.is_null() {
            atomic_saturating_add(&ALLOC_FAILURES, 1);
        } else {
            atomic_saturating_add(&BYTES_REQUESTED, new_size);
            if new_size >= old_size {
                atomic_saturating_add(&BYTES_OUTSTANDING, new_size - old_size);
            } else {
                atomic_saturating_add(&BYTES_RELEASED, old_size - new_size);
                atomic_saturating_sub(&BYTES_OUTSTANDING, old_size - new_size);
            }
        }
        new_ptr
    }
}
