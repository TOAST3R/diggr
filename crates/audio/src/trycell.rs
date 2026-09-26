//! A cell with non-blocking exclusive access.
//!
//! The audio callback uses [`TryCell::try_with`] and outputs silence if the cell is busy; the
//! supervisor uses the same call (retrying) to reconfigure the renderer after a device change.
//! Nobody ever waits inside the callback.

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct TryCell<T> {
    busy: AtomicBool,
    value: UnsafeCell<T>,
}

// SAFETY: access to `value` is serialized by the `busy` flag (acquire on take, release on give
// back), so at most one thread holds `&mut T` at a time.
unsafe impl<T: Send> Sync for TryCell<T> {}

impl<T> TryCell<T> {
    pub fn new(value: T) -> Self {
        Self {
            busy: AtomicBool::new(false),
            value: UnsafeCell::new(value),
        }
    }

    /// Runs `f` with exclusive access, or returns `None` immediately if someone else has it.
    pub fn try_with<R>(&self, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        if self
            .busy
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            return None;
        }
        struct Release<'a>(&'a AtomicBool);
        impl Drop for Release<'_> {
            fn drop(&mut self) {
                self.0.store(false, Ordering::Release);
            }
        }
        let _release = Release(&self.busy);
        // SAFETY: we hold the busy flag.
        Some(f(unsafe { &mut *self.value.get() }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn exclusive_and_non_blocking() {
        let cell = TryCell::new(0);
        let inner = cell.try_with(|v| {
            *v += 1;
            cell.try_with(|_| ())
        });
        assert_eq!(
            inner,
            Some(None),
            "nested access must be refused, not block"
        );
        assert_eq!(cell.try_with(|v| *v), Some(1));
    }

    #[test]
    fn concurrent_increments_are_not_lost() {
        let cell = Arc::new(TryCell::new(0u64));
        let threads: Vec<_> = (0..4)
            .map(|_| {
                let cell = cell.clone();
                std::thread::spawn(move || {
                    let mut done = 0;
                    while done < 10_000 {
                        if cell.try_with(|v| *v += 1).is_some() {
                            done += 1;
                        }
                    }
                })
            })
            .collect();
        threads.into_iter().for_each(|t| t.join().unwrap());
        assert_eq!(cell.try_with(|v| *v), Some(40_000));
    }
}
