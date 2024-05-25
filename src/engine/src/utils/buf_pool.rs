use std::cell::UnsafeCell;
use std::intrinsics::{likely, unlikely};
use std::{mem};
use std::fmt::Debug;
use std::mem::MaybeUninit;

pub struct Buffer {
    slice: Box<[u8]>,
    written: usize,
    offset: usize,
    from_pool: bool
}

impl Buffer {
    pub fn new(size: usize) -> Self {
        let mut v = Vec::with_capacity(size);
        unsafe { v.set_len(size) };
        Buffer {
            slice: v.into_boxed_slice(),
            written: 0,
            offset: 0,
            from_pool: false
        }
    }

    fn new_from_pool(size: usize) -> Self {
        let mut v = Vec::with_capacity(size);
        unsafe { v.set_len(size) };
        Buffer {
            slice: v.into_boxed_slice(),
            written: 0,
            offset: 0,
            from_pool: true
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.written
    }

    #[inline(always)]
    pub fn offset(&self) -> usize {
        self.offset
    }

    #[inline(always)]
    pub fn set_offset(&mut self, offset: usize) {
        self.offset = offset;
    }

    #[inline(always)]
    pub fn cap(&self) -> usize {
        self.slice.len()
    }

    // TODO: need test
    pub fn append(&mut self, buf: &[u8]) {
        let len = buf.len();
        if unlikely(len > self.slice.len() - self.written) {
            let temp = self.slice[..self.written].to_vec();
            let new_len = (temp.len() + len) * 2;
            let mut v = Vec::with_capacity(new_len);
            unsafe { v.set_len(new_len) };
            self.slice = v.into_boxed_slice();
            self.slice[..self.written].copy_from_slice(&temp);
            self.from_pool = false;
        }

        self.slice[self.written..self.written + len].copy_from_slice(buf);
        self.written += len;
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.slice[self.offset..self.written]
    }

    pub fn as_ptr(&self) -> *const u8 {
        unsafe { self.slice.as_ptr().offset(self.offset as isize) }
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        unsafe { self.slice.as_mut_ptr().offset(self.offset as isize) }
    }

    pub fn clear(&mut self) {
        self.written = 0;
        self.offset = 0;
    }

    fn release(self) {
        buf_pool().put(self);
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Buffer::new(0)
    }
}

impl Debug for Buffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.as_slice())
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        let buf = mem::take(self);
        buf.release();
    }
}

thread_local! {
    pub static BUF_POOL: UnsafeCell<MaybeUninit<BufPool>> = UnsafeCell::new(MaybeUninit::zeroed());
}

#[inline(always)]
pub(crate) fn buf_pool() -> &'static mut BufPool {
    BUF_POOL.with(|pool|
        unsafe { (&mut *pool.get()).assume_init_mut()}
    )
}

#[inline(always)]
pub fn buffer() -> Buffer {
    buf_pool().get()
}

pub struct BufPool {
    pool: Vec<Buffer>,
    buffer_len: usize
}

impl BufPool {
    // TODO init in all program if needed, not only in run_on_cores
    pub fn init(buffer_len: usize) {
        BUF_POOL.with(|pool| {
            let pool_ref = unsafe { (&mut *pool.get()).assume_init_mut() };
            *pool_ref = BufPool {
                pool: Vec::with_capacity(0),
                buffer_len
            };
        });
    }

    pub fn tune_buffer_len(&mut self, buffer_len: usize) {
        self.buffer_len = buffer_len;
        self.pool = Vec::with_capacity(0);
    }

    pub fn get(&mut self) -> Buffer {
        if unlikely(self.pool.is_empty()) {
            return Buffer::new_from_pool(self.buffer_len);
        }

        unsafe { self.pool.pop().unwrap_unchecked() }
    }

    fn put(&mut self, mut buf: Buffer) {
        if likely(buf.from_pool) {
            buf.clear();
            self.pool.push(buf);
        }
    }
}