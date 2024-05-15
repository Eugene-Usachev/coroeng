use std::fmt;
use std::fmt::{Debug, Display};
use std::ops::{Deref, DerefMut};
use crate::engine::sync::Locker;

pub struct LockerGuard<'a, T, L: Locker<T>> {
    locker: &'a L
}

impl<'a, T, L: Locker<T>> LockerGuard<'a, T, L> {
    pub fn new(locker: &'a L) -> LockerGuard<'a, T, L> {
        LockerGuard { locker }
    }
}

impl<'a, T, L: Locker<T>> Drop for LockerGuard<'a, T, L> {
    fn drop(&mut self) {
        unsafe { self.locker.unlock(); }
    }
}

impl<T: ?Sized, L: Locker<T>> !Send for LockerGuard<'_, T, L> {}
unsafe impl<T: ?Sized + Sync, L: Locker<T>> Sync for LockerGuard<'_, T, L> {}

impl<T: ?Sized, L: Locker<T>> Deref for LockerGuard<'_, T, L> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.locker.as_ref() }
    }
}

impl<T: ?Sized, L: Locker<T>> DerefMut for LockerGuard<'_, T, L> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.locker.as_mut() }
    }
}

impl<T: ?Sized + Debug, L: Locker<T>> Debug for LockerGuard<'_, T, L> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&**self, f)
    }
}

impl<T: ?Sized + Display, L: Locker<T>> Display for LockerGuard<'_, T, L> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}