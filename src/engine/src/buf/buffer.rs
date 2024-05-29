use std::fmt::Debug;
use std::intrinsics::unlikely;
use std::mem;
use crate::buf::buf_pool::buf_pool;

/// Buffer for data transfer. Buffer is allocated in heap.
///
/// Buffer has `written` and `offset` fields.
///
/// - `written` is how many bytes have been written into the buffer. For "usual" user it is length of the buffer.
///
/// - `offset` is how many bytes have been read from the buffer. For example, it is used in [`TcpStream::write`](crate::net::TcpStream::write).
/// Use it only if you know what you are doing. In most cases it is need only for inner work.
/// 
/// # About pool
/// 
/// For get from [`BufPool`](crate::buf::BufPool) call [`buffer`](crate::buf::buffer).
/// If you can use [`BufPool`](crate::buf::BufPool), use it, to have better performance.
///
/// If it was gotten from [`BufPool`](crate::buf::BufPool) it will come back after drop.
///
/// # Buffer representation
///
/// ```text
/// +---+---+---+---+---+---+---+---+
/// | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 |
/// +---+---+---+---+---+---+---+---+
/// | X | X | X | X | X |   |   |   |
/// +---+---+---+---+---+---+---+---+
///     ^               ^           ^
///   offset         written       cap
///
/// offset = 1 (between 1 and 2)
/// written = 5 (from 1 to 5 inclusive)
/// 5 blocks occupied (X), 3 blocks free (blank)
/// ```
pub struct Buffer {
    slice: Box<[u8]>,
    written: usize,
    offset: usize,
    pub(crate) from_pool: bool
}

impl Buffer {
    /// Creates new buffer with given size. This buffer will not be put to the pool.
    /// So, use it only for creating a buffer with specific size.
    #[inline(always)]
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

    /// Creates a new buffer from a pool with the given size.
    pub(crate) fn new_from_pool(size: usize) -> Self {
        let mut v = Vec::with_capacity(size);
        unsafe { v.set_len(size) };
        Buffer {
            slice: v.into_boxed_slice(),
            written: 0,
            offset: 0,
            from_pool: true
        }
    }

    /// Returns how many bytes have been written into the buffer.
    /// For "usual" user it is length of the buffer.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.written
    }

    /// Returns how many bytes have been read from the buffer.
    /// For example, it is used in [`TcpStream::write`](crate::net::TcpStream::write).
    /// Used it only if you know what you are doing. In most cases it is need only for inner work.
    #[inline(always)]
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Sets offset.
    /// It is used in [`TcpStream::write`](crate::net::TcpStream::write).
    /// Used it only if you know what you are doing. In most cases it is need only for inner work.
    #[inline(always)]
    pub fn set_offset(&mut self, offset: usize) {
        self.offset = offset;
    }

    /// Returns capacity of the buffer.
    #[inline(always)]
    pub fn cap(&self) -> usize {
        self.slice.len()
    }

    /// Appends data to the buffer. If a capacity is not enough, the buffer will be resized and will not be put to the pool.
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

    /// Returns a slice of the buffer.
    ///
    /// # Note
    ///
    /// The slice is from `offset` to `written`.
    pub fn as_slice(&self) -> &[u8] {
        &self.slice[self.offset..self.written]
    }

    /// Returns a pointer to the buffer.
    ///
    /// # Note
    ///
    /// The pointer is shifted by `offset`.
    pub fn as_ptr(&self) -> *const u8 {
        unsafe { self.slice.as_ptr().offset(self.offset as isize) }
    }

    /// Returns a mutable pointer to the buffer.
    ///
    /// # Note
    ///
    /// The pointer is shifted by `offset`.
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        unsafe { self.slice.as_mut_ptr().offset(self.offset as isize) }
    }

    /// Clears the buffer.
    pub fn clear(&mut self) {
        self.written = 0;
        self.offset = 0;
    }

    /// Puts the buffer to the pool. You can not to use it, and then this method will be called automatically by drop.
    pub fn release(self) {
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