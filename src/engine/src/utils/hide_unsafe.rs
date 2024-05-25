use std::cell::UnsafeCell;

/// Hides the unsafe part of a cell. It looks like this:
///
/// ```ignore
/// unsafe { &*cell.get() }
/// ```
///
pub fn hide_unsafe<'a, T>(cell: &UnsafeCell<T>) -> &'a T {
    unsafe { &*cell.get() }
}

/// Hides the unsafe part of a cell. It looks like this:
///
/// ```ignore
/// unsafe { &mut *cell.get() }
/// ```
pub fn hide_mut_unsafe<'a, T>(cell: &UnsafeCell<T>) -> &'a mut T {
    unsafe { &mut *cell.get() }
}