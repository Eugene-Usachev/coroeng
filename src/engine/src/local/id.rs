use std::cell::{UnsafeCell};
use std::mem::MaybeUninit;

thread_local! {
    /// Local worker id. If it is zero, then this thread is not a worker.
    pub static WORKER_ID: UnsafeCell<MaybeUninit<usize>> = UnsafeCell::new(MaybeUninit::zeroed());
    /// Core id of this thread.
    pub static CORE_ID: UnsafeCell<MaybeUninit<usize>> = UnsafeCell::new(MaybeUninit::zeroed());
}

/// Get local worker id.
pub fn get_worker_id() -> usize {
    WORKER_ID.with(|thread_id| unsafe { (&*thread_id.get()).assume_init_read() })
}

/// Get core id of this thread.
pub fn get_core_id() -> usize {
    CORE_ID.with(|thread_id| unsafe { (&*thread_id.get()).assume_init_read() })
}

/// Set worker id and core id of this thread.
///
/// # Panics
///
/// Panics if worker id is zero.
///
pub(crate) fn set_worker_id_and_core_id(worker_id: usize, core_id: usize) {
    WORKER_ID.with(|thread_id| unsafe {
        if (&*thread_id.get()).assume_init_read() == 0 {
            if worker_id == 0 {
                panic!("[BUG] Worker id can't be 0. Please report this issue.");
            }
            (&mut *thread_id.get()).write(worker_id);
        } else {
            panic!("[BUG] Double init worker id. Please report this issue.");
        }
    });
    CORE_ID.with(|thread_id| unsafe { (&mut *thread_id.get()).write(core_id) });
}

/// Set worker id and core id to zero.
pub(crate) fn set_worker_id_and_core_id_to_zero() {
    WORKER_ID.with(|thread_id| unsafe {
        (&mut *thread_id.get()).write(0);
    });
    CORE_ID.with(|thread_id| unsafe { (&mut *thread_id.get()).write(0) });
}