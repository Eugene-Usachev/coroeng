use std::cell::UnsafeCell;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use crossbeam::utils::{CachePadded};
use crate::coroutine::YieldStatus;
use crate::sync::{Locker};
use std::fmt;
use std::fmt::{Debug, Display};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

pub struct Mutex<'a, T> {
    state: CachePadded<AtomicUsize>,
    value: UnsafeCell<T>,
    phantom: PhantomData<&'a T>
}

const UNLOCKED: usize = 0;
const LOCKED: usize = 1;

impl<'a, T> Mutex<'a, T> {
    pub const fn new(value: T) -> Self {
        Self {
            state: CachePadded::new(AtomicUsize::new(UNLOCKED)),
            value: UnsafeCell::new(value),
            phantom: PhantomData
        }
    }
}

impl<'a, T> Locker<'a, T> for Mutex<'a, T> {
    const YIELD: YieldStatus = YieldStatus::Yield;
    type Guard = MutexGuard<'a, T>;

    #[inline(always)]
    unsafe fn new_guard(&'a self) -> MutexGuard<'a, T> {
        MutexGuard::new(self)
    }

    #[inline(always)]
    unsafe fn try_unsafe_lock(&self) -> Result<(), ()> {
        if self.state.compare_exchange_weak(UNLOCKED, LOCKED, Acquire, Relaxed).is_ok() {
            Ok(())
        } else {
            Err(())
        }
    }

    #[inline(always)]
    unsafe fn unlock(&self) {
        self.state.store(UNLOCKED, Release);
    }

    #[inline(always)]
    unsafe fn as_ref(&self) -> &'a T {
        unsafe {
            &*self.value.get()
        }
    }

    #[inline(always)]
    unsafe fn as_mut(&self) -> &'a mut T {
        unsafe {
            &mut *self.value.get()
        }
    }
}

unsafe impl<'a, T> Sync for Mutex<'a, T> {}

pub struct MutexGuard<'a, T> {
    locker: &'a Mutex<'a, T>,
    phantom_data: PhantomData<T>
}

impl<'a, T> MutexGuard<'a, T> {
    pub fn new(locker: &'a Mutex<T>) -> MutexGuard<'a, T> {
        MutexGuard { locker, phantom_data: PhantomData }
    }
}

impl<'a, T> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        unsafe { self.locker.unlock(); }
    }
}

impl<T> !Send for MutexGuard<'_, T> {}
unsafe impl<T: Sync> Sync for MutexGuard<'_, T> {}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.locker.as_ref() }
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.locker.as_mut() }
    }
}

impl<T: Debug> Debug for MutexGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&**self, f)
    }
}

impl<T: Display> Display for MutexGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}