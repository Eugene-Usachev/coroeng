use std::cell::UnsafeCell;

pub fn hide_unsafe<'a, T>(cell: &UnsafeCell<T>) -> &'a T {
    unsafe { &*cell.get() }
}

pub fn hide_mut_unsafe<'a, T>(cell: &UnsafeCell<T>) -> &'a mut T {
    unsafe { &mut *cell.get() }
}