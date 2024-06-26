use std::fmt::Debug;
use std::intrinsics::unlikely;
use std::io::{Read, Write};
use std::{cmp};
use std::alloc::{alloc, dealloc, Layout};
use std::cmp::{max};
use std::ptr::{NonNull, slice_from_raw_parts_mut};
use crate::buf::buf_pool::buf_pool;
use crate::buf::buffer;
use crate::cfg::config_buf_len;

/// Buffer for data transfer. Buffer is allocated in heap.
///
/// Buffer has `written` and `offset` fields.
///
/// - `len` is how many bytes have been written into the buffer.
///
/// - `offset` is how many bytes have been read from the buffer. For example, it is used in [`TcpStream::write`](crate::net::TcpStream::write).
/// Use it only if you know what you are doing. In most cases it is need only for inner work.
/// 
/// # About pool
/// 
/// For get from [`BufPool`] call [`buffer`](crate::buf::buffer).
/// If you can use [`BufPool`], use it, to have better performance.
///
/// If it was gotten from [`BufPool`] it will come back after drop.
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
///   offset           len         cap
///
/// offset = 1 (between 1 and 2)
/// len = 5 (from 1 to 5 inclusive)
/// 5 blocks occupied (X), 3 blocks free (blank)
/// ```
/// 
/// # Example of representation
///
/// ## Start
/// 
/// ```text
/// +---+---+---+---+---+---+---+---+
/// | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 |
/// +---+---+---+---+---+---+---+---+
/// |   |   |   |   |   |   |   |   |
/// +---+---+---+---+---+---+---+---+
/// ^                               ^
/// offset and len                 cap
/// ```
/// 
/// ## After [`append`](Buffer::append)
/// 
/// ```text
/// +---+---+---+---+---+---+---+---+
/// | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 |
/// +---+---+---+---+---+---+---+---+
/// | X | X | X | X |   |   |   |   |
/// +---+---+---+---+---+---+---+---+
/// ^               ^               ^
/// offset         len             cap
/// ```
/// 
/// ## After [`write`](crate::io::AsyncWrite::write) operation
/// 
/// ```text
/// +---+---+---+---+---+---+---+---+
/// | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 |
/// +---+---+---+---+---+---+---+---+
/// | X | X | X | X |   |   |   |   |
/// +---+---+---+---+---+---+---+---+
///         ^       ^               ^
///       offset   len             cap
/// ```
/// 
/// ## After [`write_all`](crate::io::AsyncWrite::write_all) operation. This state returns `true` in [`written_full`](Buffer::written_full)
/// 
/// ```text
/// +---+---+---+---+---+---+---+---+
/// | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 |
/// +---+---+---+---+---+---+---+---+
/// | X | X | X | X |   |   |   |   |
/// +---+---+---+---+---+---+---+---+
///                 ^               ^
///         offset and len         cap
/// ```
/// 
/// [`BufPool`]: crate::buf::BufPool
pub struct Buffer {
    pub(crate) slice: NonNull<[u8]>,
    len: usize,
    offset: usize
}

impl Buffer {
    /// Creates raw slice with given capacity. This code avoids checking zero capacity.
    ///
    /// # Safety
    ///
    /// capacity > 0
    #[inline(always)]
    fn raw_slice(capacity: usize) -> NonNull<[u8]> {
        let layout = match Layout::array::<u8>(capacity) {
            Ok(layout) => layout,
            Err(_) => panic!("Cannot create slice with capacity {capacity}. Capacity overflow."),
        };
        unsafe {
            NonNull::new_unchecked(slice_from_raw_parts_mut(alloc(layout), capacity))
        }
    }

    /// Creates new buffer with given size. This buffer will not be put to the pool.
    /// So, use it only for creating a buffer with specific size.
    ///
    /// # Safety
    /// - size > 0
    #[inline(always)]
    pub fn new(size: usize) -> Self {
        if unlikely(size == 0) {
            panic!("Cannot create Buffer with size 0. Size must be > 0.");
        }

        Buffer {
            slice: Self::raw_slice(size),
            len: 0,
            offset: 0
        }
    }

    /// Creates a new buffer from a pool with the given size.
    #[inline(always)]
    pub(crate) fn new_from_pool(size: usize) -> Self {
        Buffer {
            slice: Self::raw_slice(size),
            len: 0,
            offset: 0
        }
    }

    /// Returns `true` if the buffer is empty.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns how many bytes have been written into the buffer, exclusive offset.
    /// So, it is [`len`](#field.len) - [`offset`](#field.offset).
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len - self.offset
    }

    /// Increases [`len`](#field.len) on diff
    #[inline(always)]
    pub fn add_len(&mut self, diff: usize) {
        self.len += diff;
    }

    /// Sets [`len`](#field.len).
    #[inline(always)]
    pub fn set_len(&mut self, written: usize) {
        self.len = written;
    }

    /// Returns how many bytes have been read from the buffer.
    /// For example, it is used in [`TcpStream::write`](crate::net::TcpStream::write).
    /// Used it only if you know what you are doing. In most cases it is need only for inner work.
    #[inline(always)]
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Sets [`offset`](#field.offset).
    /// It is used in [`TcpStream::write`](crate::net::TcpStream::write).
    /// Used it only if you know what you are doing. In most cases it is need only for inner work.
    #[inline(always)]
    pub fn set_offset(&mut self, offset: usize) {
        self.offset = offset;
    }
    
    /// Increases [`offset`](#field.offset) on diff
    #[inline(always)]
    pub fn add_offset(&mut self, diff: usize) {
        self.offset += diff;
    }
    
    /// Sets [`offset`](#field.offset) to [`len`](#field.len).
    /// 
    /// It means that all buffer data has been written. So, after it [`written_full`](Buffer::written_full) returns `true`.
    // TODO add doc where it is used after read to second read. After it, maybe update AsyncRead docs.
    #[inline(always)]
    pub fn set_offset_to_len(&mut self) {
        self.offset = self.len;
    }

    /// Returns a real capacity of the buffer.
    #[inline(always)]
    pub fn real_cap(&self) -> usize {
        self.slice.len()
    }
    
    /// Returns a capacity of the buffer (real capacity - offset).
    #[inline(always)]
    pub fn cap(&self) -> usize {
        self.real_cap() - self.offset
    }
    
    /// Returns `true` if all data has been written. 
    /// 
    /// It is equal to [`offset`](#field.offset) == [`len`](#field.len).
    #[inline(always)]
    pub fn written_full(&self) -> bool {
        self.offset == self.len
    }

    #[inline(always)]
    fn resize(&mut self, new_size: usize) {
        if new_size < self.len {
            self.len = new_size;
        }

        let mut new_buf = if config_buf_len() == new_size {
            buffer()
        } else {
            Buffer::new(new_size)
        };

        new_buf.len = self.len;
        new_buf.as_mut()[..self.len].copy_from_slice(&self.as_ref()[..self.len]);

        *self = new_buf;
    }

    /// Appends data to the buffer. If a capacity is not enough, the buffer will be resized and will not be put to the pool.
    // TODO: need test
    #[inline(always)]
    pub fn append(&mut self, buf: &[u8]) {
        let len = buf.len();
        if unlikely(len > self.slice.len() - self.len) {
            self.resize(max(self.len + len, self.real_cap() * 2));
        }

        unsafe { self.slice.as_mut()[self.len..self.len + len].copy_from_slice(buf); }
        self.len += len;
    }

    /// Returns a pointer to the buffer.
    ///
    /// # Note
    ///
    /// The pointer is shifted by [`offset`](#field.offset).
    #[inline(always)]
    pub fn as_ptr(&self) -> *const u8 {
        unsafe { self.slice.as_ptr().as_mut_ptr().offset(self.offset as isize) }
    }

    /// Returns a mutable pointer to the buffer.
    ///
    /// # Note
    ///
    /// The pointer is shifted by [`offset`](#field.offset).
    #[inline(always)]
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        unsafe { self.slice.as_mut_ptr().offset(self.offset as isize) }
    }

    /// Clears the buffer.
    #[inline(always)]
    pub fn clear(&mut self) {
        self.len = 0;
        self.offset = 0;
    }

    /// Puts the buffer to the pool. You can not to use it, and then this method will be called automatically by drop.
    #[inline(always)]
    pub fn release(self) {
        buf_pool().put(self);
    }

    /// Puts the buffer to the pool without checking for a size.
    ///
    /// # Safety
    /// - [`buf.real_cap`](#method.real_cap)() ==[` config_buf_len`](config_buf_len)()
    #[inline(always)]
    pub unsafe fn release_unchecked(self) {
        unsafe { buf_pool().put_unchecked(self) };
    }
}

impl Write for Buffer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.append(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Read for Buffer {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let len = cmp::min(buf.len(), self.len - self.offset);
        buf[..len].copy_from_slice(&self.as_ref()[self.offset..self.offset + len]);
        self.offset += len;
        Ok(len)
    }
}

impl AsRef<[u8]> for Buffer {
    fn as_ref(&self) -> &[u8] {
        unsafe { &self.slice.as_ref()[self.offset..self.len] }
    }
}

impl AsMut<[u8]> for Buffer {
    fn as_mut(&mut self) -> &mut [u8] {
        unsafe { &mut self.slice.as_mut()[self.offset..self.len] }
    }
}

impl Debug for Buffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.as_ref())
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        if self.real_cap() == config_buf_len() {
            let buf = Buffer {
                slice: self.slice,
                len: self.len,
                offset: self.offset
            };
            unsafe { buf.release_unchecked() };
        } else {
            unsafe { dealloc(self.slice.as_mut_ptr(), Layout::array::<u8>(self.real_cap()).unwrap_unchecked())}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let buf = Buffer::new(1);
        assert_eq!(buf.cap(), 1);
    }

    #[test]
    #[should_panic(expected = "Cannot create Buffer with size 0. Size must be > 0.")]
    fn test_new_panic() {
        Buffer::new(0);
    }

    #[test]
    fn test_add_len_and_add_offset_and_set_offset_to_len() {
        let mut buf = Buffer::new(100);
        assert_eq!(buf.len(), 0);
        assert_eq!(buf.offset(), 0);

        buf.add_len(10);
        assert_eq!(buf.len(), 10);
        assert_eq!(buf.offset(), 0);

        buf.add_len(20);
        assert_eq!(buf.len(), 30);
        
        buf.add_offset(19);
        assert_eq!(buf.len(), 11);
        assert_eq!(buf.offset(), 19);
        
        buf.set_offset_to_len();
        assert_eq!(buf.len(), 0);
        assert_eq!(buf.offset(), 30);
        assert!(buf.written_full());
    }

    #[test]
    fn test_len_and_cap_and_offset_and_real_cap() {
        let mut buf = Buffer::new(100);
        assert_eq!(buf.offset(), 0);
        assert_eq!(buf.len(), 0);
        assert_eq!(buf.cap(), 100);

        buf.set_len(10);
        assert_eq!(buf.offset(), 0);
        assert_eq!(buf.len(), 10);
        assert_eq!(buf.cap(), 100);

        buf.set_offset(10);
        assert_eq!(buf.offset(), 10);
        assert_eq!(buf.len(), 0);
        assert_eq!(buf.cap(), 90);
        assert_eq!(buf.real_cap(), 100);
    }

    #[test]
    fn test_resize() {
        let mut buf = Buffer::new(100);

        buf.resize(200);
        assert_eq!(buf.cap(), 200);

        buf.resize(50);
        assert_eq!(buf.cap(), 50);
    }

    #[test]
    fn test_append_and_offset_and_clear() {
        let mut buf = Buffer::new(5);

        buf.append(&[1, 2, 3]);
        // This code checks written
        assert_eq!(buf.as_ref(), &[1, 2, 3]);
        assert_eq!(buf.cap(), 5);

        buf.append(&[4, 5, 6]);
        assert_eq!(buf.as_ref(), &[1, 2, 3, 4, 5, 6]);
        assert_eq!(buf.cap(), 10);

        buf.set_offset(2);
        buf.append(&[7, 8, 9]);
        assert_eq!(buf.as_ref(), &[3, 4, 5, 6, 7, 8, 9]);
        assert_eq!(buf.cap(), 8);

        buf.clear();
        assert_eq!(buf.as_ref(), &[]);
        assert_eq!(buf.cap(), 10);
        assert_eq!(buf.offset(), 0);
    }

    #[test]
    fn test_is_empty() {
        let mut buf = Buffer::new(5);
        assert!(buf.is_empty());
        buf.append(&[1, 2, 3]);
        assert!(!buf.is_empty());
        buf.clear();
        assert!(buf.is_empty());
    }
}