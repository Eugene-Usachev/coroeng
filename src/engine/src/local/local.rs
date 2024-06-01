// TODO docs and tests and make it pub in mod.rs

use std::fmt::Debug;
use crate::local::get_worker_id;
use crate::utils::Ptr;

pub struct Local<T> {
    worker_id: usize,
    data: Ptr<T>,
    counter: Ptr<usize>
}

impl<T> Local<T> {
    pub fn new(data: T) -> Self {
        let worker_id = get_worker_id();
        if worker_id == 0 {
            panic!("Cannot create local data a thread, that has not start Scheduler! \
            Please call run_on_core on run_on_all_cores first.");
        }

        Local {
            worker_id,
            data: Ptr::new(data),
            counter: Ptr::new(1)
        }
    }

    #[inline(always)]
    fn inc_counter(&self) {
        unsafe {
            *&mut *self.counter.as_ptr() += 1;
        }
    }

    #[inline(always)]
    fn dec_counter(&self) -> usize {
        unsafe {
            let reference = &mut *self.counter.as_ptr();
            let new = *reference - 1;
            *reference = new;
            new
        }
    }

    #[inline(always)]
    pub fn check_worker_id(&self) -> bool {
        self.worker_id == get_worker_id()
    }

    #[inline(always)]
    pub fn get<'a>(&self) -> &'a T {
        if self.check_worker_id() {
            unsafe { self.data.as_ref() }
        } else {
            panic!("Tried to get local data from another worker!");
        }
    }

    #[inline(always)]
    pub fn get_mut<'a>(&self) -> &'a mut T {
        if self.check_worker_id() {
            unsafe { self.data.as_mut() }
        } else {
            panic!("Tried to get_mut local data from another worker!");
        }
    }
}

impl<T: Default> Default for Local<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: Debug> Debug for Local<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.get().fmt(f)
    }
}

impl<T: Clone> Clone for Local<T> {
    fn clone(&self) -> Self {
        self.inc_counter();
        Self {
            worker_id: self.worker_id,
            data: self.data,
            counter: self.counter
        }
    }
}

impl<T> Drop for Local<T> {
    fn drop(&mut self) {
        if self.dec_counter() == 0 {
            unsafe {
                self.data.drop_in_place();
                self.counter.drop_in_place();
            }
        }
    }
}

impl<T: Copy> Copy for Local<T> {}

unsafe impl<T> Send for Local<T> {}
unsafe impl<T> Sync for Local<T> {}