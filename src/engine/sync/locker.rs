use crate::engine::sync::guard::LockerGuard;

pub trait Locker<T> {
    unsafe fn unsafe_lock(&self);
    fn lock(&self) -> LockerGuard<'_, T, Self<T>> {
        unsafe {
            self.unsafe_lock();
        }
        LockerGuard::new(self)
    }

    unsafe fn unlock(&self);
    unsafe fn as_ref<'a>(&self) -> &'a T;
    unsafe fn as_mut<'a>(&self) -> &'a mut T;
}