use std::fmt::Debug;
use std::{ptr};
use std::alloc::{alloc, dealloc, Layout};

/// A pointer wrapper.
pub struct Ptr<T> {
    ptr: *mut T
}

impl<T> Ptr<T> {
    /// Create a new `Ptr` with the given value.
    #[inline(always)]
    pub fn new(value: T) -> Self {
        let ptr = unsafe { alloc(Layout::new::<T>()) } as *mut T;
        unsafe { ptr.write(value) };
        Self {
            ptr
        }
    }

    /// Create a null `Ptr`.
    #[inline(always)]
    pub fn null() -> Self {
        Self {
            ptr: ptr::null_mut()
        }
    }

    /// Check if the pointer is null.
    #[inline(always)]
    pub fn is_null(&self) -> bool {
        self.ptr.is_null()
    }

    /// Get the raw pointer.
    #[inline(always)]
    pub fn as_ptr(&self) -> *mut T {
        self.ptr
    }

    /// Get a reference to the value.
    ///
    /// # Panics
    ///
    /// If the pointer is null.
    #[inline(always)]
    pub unsafe fn as_ref<'a>(self) -> &'a T {
        if self.ptr.is_null() {
            panic!("ptr is null");
        }
        unsafe { &*self.ptr }
    }

    /// Get a mutable reference to the value.
    ///
    /// # Panics
    ///
    /// If the pointer is null.
    #[inline(always)]
    pub unsafe fn as_mut<'a>(self) -> &'a mut T {
        if self.ptr.is_null() {
            panic!("ptr is null");
        }
        unsafe { &mut *self.ptr }
    }

    /// Returns the pointer as an u64.
    /// Use [`Ptr::from`](#method.from) to convert it back.
    #[inline(always)]
    pub fn as_u64(&self) -> u64 {
        self.ptr as u64
    }

    /// Drop the value.
    ///
    /// # Panics
    ///
    /// If the pointer is null.
    #[inline(always)]
    pub unsafe fn drop_in_place(self) {
        if self.ptr.is_null() {
            println!("ptr is null");
            return;
        }

        unsafe {
            dealloc(self.ptr as *mut u8, Layout::new::<T>());
        }
    }

    /// Return the value. It will not lead to the pointer value being dropped.
    ///
    /// # Panics
    ///
    /// If the pointer is null.
    #[inline(always)]
    pub unsafe fn read(self) -> T {
        unsafe { ptr::read(self.ptr) }
    }

    /// Set the value. Does not call drop the old value.
    ///
    /// # Panics
    ///
    /// If the pointer is null.
    #[inline(always)]
    pub unsafe fn write(self, value: T) {
        unsafe { ptr::write(self.ptr, value) }
    }
}

impl<T> Clone for Ptr<T> {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr
        }
    }
}

impl<T> Copy for Ptr<T> {}

unsafe impl<T: Send> Send for Ptr<T> {}
unsafe impl<T: Sync> Sync for Ptr<T> {}

impl<T> From<usize> for Ptr<T> {
    fn from(ptr: usize) -> Self {
        Self {
            ptr: ptr as *mut T
        }
    }
}

impl<T> From<u64> for Ptr<T> {
    fn from(ptr: u64) -> Self {
        Self {
            ptr: ptr as *mut T
        }
    }
}

impl<T> From<&mut T> for Ptr<T> {
    fn from(ptr: &mut T) -> Self {
        Self {
            ptr
        }
    }
}

impl<T: Debug> Debug for Ptr<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe { write!(f, "{:?}", self.as_ref()) }
    }
}