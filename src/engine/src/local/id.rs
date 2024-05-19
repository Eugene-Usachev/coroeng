// TODO docs
use std::cell::{UnsafeCell};
use std::mem::MaybeUninit;

thread_local! {
    pub static WORKER_ID: UnsafeCell<MaybeUninit<usize>> = UnsafeCell::new(MaybeUninit::zeroed());
    pub static CORE_ID: UnsafeCell<MaybeUninit<usize>> = UnsafeCell::new(MaybeUninit::zeroed());
}

pub fn get_worker_id() -> usize {
    WORKER_ID.with(|thread_id| unsafe { (&*thread_id.get()).assume_init_read() })
}

pub fn get_core_id() -> usize {
    CORE_ID.with(|thread_id| unsafe { (&*thread_id.get()).assume_init_read() })
}

pub(crate) fn set_worker_id_and_core_id(worker_id: usize, core_id: usize) {
    WORKER_ID.with(|thread_id| unsafe {
        if (&*thread_id.get()).assume_init_read() == 0 {
            if worker_id == 0 {
                panic!("[BUG] Worker id can't be 0. Please report this issue.");
            }
            (&mut *thread_id.get()).write(worker_id);
        }
    });
    CORE_ID.with(|thread_id| unsafe { (&mut *thread_id.get()).write(core_id) });
}