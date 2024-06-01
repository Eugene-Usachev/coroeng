// TODO docs

// TODO run tests (tests had been written, but hadn't been run)

// TODO make it pub in mod.rs

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
        if self.worker_id != get_worker_id() {
            panic!("Tried to inc_counter from another worker!");
        }

        unsafe {
            *&mut *self.counter.as_ptr() += 1;
        }
    }

    #[inline(always)]
    fn dec_counter(&self) -> usize {
        if self.worker_id != get_worker_id() {
            panic!("Tried to dec_counter from another worker!");
        }

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

unsafe impl<T> Send for Local<T> {}
unsafe impl<T> Sync for Local<T> {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::sync::atomic::{AtomicUsize, Ordering};

    thread_local! {
        static WORKER_ID: RefCell<usize> = RefCell::new(1);
    }

    fn set_worker_id(id: usize) {
        WORKER_ID.with(|worker_id| {
            *worker_id.borrow_mut() = id;
        });
    }

    fn mock_get_worker_id() -> usize {
        WORKER_ID.with(|worker_id| {
            *worker_id.borrow()
        })
    }

    #[test]
    #[should_panic(expected = "Cannot create local data a thread, that has not start Scheduler!")]
    fn test_new_panic_on_zero_worker_id() {
        set_worker_id(0);
        Local::new(5);
    }

    #[test]
    fn test_new_success() {
        set_worker_id(1);
        let local = Local::new(5);
        assert_eq!(local.worker_id, 1);
        assert_eq!(unsafe { *local.data.as_ptr() }, 5);
        assert_eq!(unsafe { *local.counter.as_ptr() }, 1);
    }

    #[test]
    #[should_panic(expected = "Tried to inc_counter from another worker!")]
    fn test_inc_counter_panic_on_wrong_worker_id() {
        set_worker_id(1);
        let local = Local::new(5);
        set_worker_id(2);
        local.inc_counter();
    }

    #[test]
    fn test_inc_counter() {
        set_worker_id(1);
        let local = Local::new(5);
        local.inc_counter();
        assert_eq!(unsafe { *local.counter.as_ptr() }, 2);
    }

    #[test]
    #[should_panic(expected = "Tried to dec_counter from another worker!")]
    fn test_dec_counter_panic_on_wrong_worker_id() {
        set_worker_id(1);
        let local = Local::new(5);
        set_worker_id(2);
        local.dec_counter();
    }

    #[test]
    fn test_dec_counter() {
        set_worker_id(1);
        let local = Local::new(5);
        assert_eq!(local.dec_counter(), 0);
    }

    #[test]
    fn test_check_worker_id() {
        set_worker_id(1);
        let local = Local::new(5);
        assert!(local.check_worker_id());
    }

    #[test]
    #[should_panic(expected = "Tried to get local data from another worker!")]
    fn test_get_panic_on_wrong_worker_id() {
        set_worker_id(1);
        let local = Local::new(5);
        set_worker_id(2);
        local.get();
    }

    #[test]
    fn test_get_success() {
        set_worker_id(1);
        let local = Local::new(5);
        assert_eq!(*local.get(), 5);
    }

    #[test]
    #[should_panic(expected = "Tried to get_mut local data from another worker!")]
    fn test_get_mut_panic_on_wrong_worker_id() {
        set_worker_id(1);
        let local = Local::new(5);
        set_worker_id(2);
        local.get_mut();
    }

    #[test]
    fn test_get_mut_success() {
        set_worker_id(1);
        let local = Local::new(5);
        *local.get_mut() = 10;
        assert_eq!(*local.get(), 10);
    }

    #[test]
    fn test_default() {
        set_worker_id(1);
        let local: Local<i32> = Local::default();
        assert_eq!(*local.get(), 0);
    }

    #[test]
    fn test_debug() {
        set_worker_id(1);
        let local = Local::new(5);
        assert_eq!(format!("{:?}", local), "5");
    }

    #[test]
    fn test_clone() {
        set_worker_id(1);
        let local = Local::new(5);
        let local_clone = local.clone();
        assert_eq!(unsafe { *local.counter.as_ptr() }, 2);
        assert_eq!(local.worker_id, local_clone.worker_id);
        assert_eq!(unsafe { *local.data.as_ptr() }, unsafe { *local_clone.data.as_ptr() });
    }

    #[test]
    fn test_drop() {
        set_worker_id(1);
        let local = Local::new(5);
        let local_ptr = &local as *const Local<i32>;

        // Check initial state
        unsafe { assert_eq!(*(&*local_ptr).get(), 5); }
        assert_eq!(unsafe { *&(*local_ptr).counter.as_ptr() }, 1);

        // Drop the instance
        drop(local);

        // Trying to access data after drop should panic
        let result = std::panic::catch_unwind(move || {
            let local = unsafe { &*local_ptr };
            set_worker_id(1); // Ensure correct worker ID
            // Try to access data which should be dropped
            let _ = local.get();
        });

        assert!(result.is_err());
    }
}