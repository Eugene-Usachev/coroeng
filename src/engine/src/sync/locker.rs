use std::intrinsics::unlikely;
use std::ops::{Deref, DerefMut};
use crate::coroutine::YieldStatus;
use crate::sync::spin::spin;

const TRIES: usize = 10;

pub trait Locker<'a, T: ?Sized>: Sync {
    const YIELD: YieldStatus;
    type Guard: Deref<Target = T> + DerefMut;
    unsafe fn new_guard(&'a self) -> Self::Guard;

    unsafe fn try_unsafe_lock(&self) -> Result<(), ()>;

    unsafe fn unsafe_lock(&self) -> Result<(), YieldStatus> {
        unsafe {
            let mut step = 0;
            loop {
                if self.try_unsafe_lock().is_ok() {
                    return Ok(());
                }

                spin(step);
                if unlikely(step == TRIES) {
                    return Err(Self::YIELD);
                } else {
                    step += 1;
                }
            }
        }
    }

    fn try_lock(&'a self) -> Result<Self::Guard, ()> {
        unsafe {
            match self.try_unsafe_lock() {
                Ok(()) => Ok(Self::new_guard(self)),
                Err(()) => Err(()),
            }
        }
    }
    fn lock(&'a self) -> Result<Self::Guard, YieldStatus> {
        unsafe {
            match self.unsafe_lock() {
                Ok(()) => Ok(Self::new_guard(self)),
                Err(yield_status) => {
                    return Err(yield_status);
                }
            }
        }
    }

    unsafe fn unlock(&self);
    unsafe fn as_ref(&self) -> &'a T;
    unsafe fn as_mut(&self) -> &'a mut T;
}