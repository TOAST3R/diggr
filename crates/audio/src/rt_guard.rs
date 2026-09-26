//! Detects heap allocation on the real-time audio thread.
//!
//! Install [`GuardAlloc`] as the global allocator (tests, debug builds of apps). The renderer
//! marks its callback with [`enter`]; any allocation while marked increments [`violations`].
//! Without the allocator installed, marking costs one thread-local write and detects nothing.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicU64, Ordering};

thread_local! {
    static IN_RT: Cell<bool> = const { Cell::new(false) };
}

static VIOLATIONS: AtomicU64 = AtomicU64::new(0);

/// Marks the current thread as real-time until dropped.
pub struct RtScope {
    prev: bool,
}

pub fn enter() -> RtScope {
    let prev = IN_RT.with(|f| f.replace(true));
    RtScope { prev }
}

impl Drop for RtScope {
    fn drop(&mut self) {
        IN_RT.with(|f| f.set(self.prev));
    }
}

/// Number of allocations observed inside real-time scopes since process start.
pub fn violations() -> u64 {
    VIOLATIONS.load(Ordering::Relaxed)
}

fn note() {
    // `try_with` so allocations during thread teardown never panic.
    if IN_RT.try_with(|f| f.get()).unwrap_or(false) {
        VIOLATIONS.fetch_add(1, Ordering::Relaxed);
    }
}

/// System allocator that counts allocations made inside real-time scopes.
pub struct GuardAlloc;

unsafe impl GlobalAlloc for GuardAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        note();
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        note();
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        note();
        unsafe { System.realloc(ptr, layout, new_size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        note();
        unsafe { System.dealloc(ptr, layout) }
    }
}
