use std::cell::UnsafeCell;
use std::intrinsics::{likely, unlikely};
use std::mem::MaybeUninit;
use crate::buf::Buffer;

thread_local! {
    /// Local [`BufPool`]. So, it is lockless.
    pub static BUF_POOL: UnsafeCell<MaybeUninit<BufPool>> = UnsafeCell::new(MaybeUninit::uninit());
}

/// Get [`BufPool`] from thread local. So, it is lockless.
#[inline(always)]
pub fn buf_pool() -> &'static mut BufPool {
    BUF_POOL.with(|pool|
        unsafe { (&mut *pool.get()).assume_init_mut()}
    )
}

/// Get [`Buffer`] from local [`BufPool`]. Please, do not keep the buffer longer than necessary. After drop, it will be returned to the pool.
#[inline(always)]
pub fn buffer() -> Buffer {
    buf_pool().get()
}


/// Pool of [`Buffer`]s. It is used for reusing memory. If you need to change default buffer size, use [`BufPool::tune_buffer_len`].
pub struct BufPool {
    pool: Vec<Buffer>,
    buffer_len: usize
}

impl BufPool {
    /// Initialize [`BufPool`] in local thread.
    pub fn init_in_local_thread(buffer_len: usize) {
        BUF_POOL.with(|pool| {
            let pool_ref = unsafe { &mut *pool.get() };
            *pool_ref = MaybeUninit::new(BufPool {
                pool: Vec::with_capacity(0),
                buffer_len
            });
        });
    }

    /// Uninitialize [`BufPool`] in local thread.
    pub(crate) fn uninit_in_local_thread() {
        BUF_POOL.with(|pool| {
            unsafe { (&mut *pool.get()).assume_init_drop()};
        });
    }

    /// Change default buffer size.
    pub fn tune_buffer_len(&mut self, buffer_len: usize) {
        self.buffer_len = buffer_len;
        self.pool = Vec::with_capacity(0);
    }

    /// Get [`Buffer`] from [`BufPool`].
    pub fn get(&mut self) -> Buffer {
        if unlikely(self.pool.is_empty()) {
            return Buffer::new_from_pool(self.buffer_len);
        }

        unsafe { self.pool.pop().unwrap_unchecked() }
    }

    /// Put [`Buffer`] to [`BufPool`].
    pub fn put(&mut self, buf: Buffer) {
        if likely(buf.real_cap() == self.buffer_len) {
            unsafe { self.put_unchecked(buf); }
        }
    }

    /// Put [`Buffer`] to [`BufPool`] without checking for a size.
    ///
    /// # Safety
    /// - buf.cap() == self.buffer_len
    #[inline(always)]
    pub unsafe fn put_unchecked(&mut self, mut buf: Buffer) {
        buf.clear();
        self.pool.push(buf);
    }
}