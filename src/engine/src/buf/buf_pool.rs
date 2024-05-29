use std::cell::UnsafeCell;
use std::intrinsics::{likely, unlikely};
use std::mem::MaybeUninit;
use crate::buf::Buffer;

thread_local! {
    /// Local [`BufPool`]. So, it is lockless.
    pub static BUF_POOL: UnsafeCell<MaybeUninit<BufPool>> = UnsafeCell::new(MaybeUninit::zeroed());
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
    // TODO init in all program if needed, not only in run_on_cores
    pub(crate) fn init(buffer_len: usize) {
        BUF_POOL.with(|pool| {
            let pool_ref = unsafe { (&mut *pool.get()).assume_init_mut() };
            *pool_ref = BufPool {
                pool: Vec::with_capacity(0),
                buffer_len
            };
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
    pub fn put(&mut self, mut buf: Buffer) {
        if likely(buf.from_pool) {
            buf.clear();
            self.pool.push(buf);
        }
    }
}