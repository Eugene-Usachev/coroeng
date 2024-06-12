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
/// - `written` is how many bytes have been written into the buffer. For "usual" user it is length of the buffer.
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
///   offset         written       cap
///
/// offset = 1 (between 1 and 2)
/// written = 5 (from 1 to 5 inclusive)
/// 5 blocks occupied (X), 3 blocks free (blank)
/// ```
/// 
/// [`BufPool`]: crate::buf::BufPool
pub struct Buffer {
    pub(crate) slice: NonNull<[u8]>,
    written: usize,
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
            written: 0,
            offset: 0
        }
    }

    /// Creates a new buffer from a pool with the given size.
    #[inline(always)]
    pub(crate) fn new_from_pool(size: usize) -> Self {
        Buffer {
            slice: Self::raw_slice(size),
            written: 0,
            offset: 0
        }
    }

    /// Returns `true` if the buffer is empty.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.written == 0
    }

    /// Returns how many bytes have been written into the buffer, exclusive offset.
    /// So, it is `written` - `offset`.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.written - self.offset
    }

    /// Increases written on diff
    #[inline(always)]
    pub fn add_written(&mut self, diff: usize) {
        self.written += diff;
    }

    /// Sets written.
    #[inline(always)]
    pub fn set_written(&mut self, written: usize) {
        self.written = written;
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

    #[inline(always)]
    fn resize(&mut self, new_size: usize) {
        if new_size < self.written {
            self.written = new_size;
        }

        let mut new_buf = if config_buf_len() == new_size {
            buffer()
        } else {
            Buffer::new(new_size)
        };

        new_buf.written = self.written;
        new_buf.as_mut()[..self.written].copy_from_slice(&self.as_ref()[..self.written]);

        *self = new_buf;
    }

    /// Appends data to the buffer. If a capacity is not enough, the buffer will be resized and will not be put to the pool.
    // TODO: need test
    #[inline(always)]
    pub fn append(&mut self, buf: &[u8]) {
        let len = buf.len();
        if unlikely(len > self.slice.len() - self.written) {
            self.resize(max(self.written + len, self.real_cap() * 2));
        }

        unsafe { self.slice.as_mut()[self.written..self.written + len].copy_from_slice(buf); }
        self.written += len;
    }

    /// Returns a pointer to the buffer.
    ///
    /// # Note
    ///
    /// The pointer is shifted by `offset`.
    #[inline(always)]
    pub fn as_ptr(&self) -> *const u8 {
        unsafe { self.slice.as_ptr().as_mut_ptr().offset(self.offset as isize) }
    }

    /// Returns a mutable pointer to the buffer.
    ///
    /// # Note
    ///
    /// The pointer is shifted by `offset`.
    #[inline(always)]
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        unsafe { self.slice.as_mut_ptr().offset(self.offset as isize) }
    }

    /// Clears the buffer.
    #[inline(always)]
    pub fn clear(&mut self) {
        self.written = 0;
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
        let len = cmp::min(buf.len(), self.written - self.offset);
        buf[..len].copy_from_slice(&self.as_ref()[self.offset..self.offset + len]);
        self.offset += len;
        Ok(len)
    }
}

impl AsRef<[u8]> for Buffer {
    fn as_ref(&self) -> &[u8] {
        unsafe { &self.slice.as_ref()[self.offset..self.written] }
    }
}

impl AsMut<[u8]> for Buffer {
    fn as_mut(&mut self) -> &mut [u8] {
        unsafe { &mut self.slice.as_mut()[self.offset..self.written] }
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
                written: self.written,
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
    fn test_add_written() {
        let mut buf = Buffer::new(100);
        assert_eq!(buf.len(), 0);

        buf.add_written(10);
        assert_eq!(buf.len(), 10);

        buf.add_written(20);
        assert_eq!(buf.len(), 30);
    }

    #[test]
    fn test_len_and_cap_and_offset_and_real_cap() {
        let mut buf = Buffer::new(100);
        assert_eq!(buf.offset(), 0);
        assert_eq!(buf.len(), 0);
        assert_eq!(buf.cap(), 100);

        buf.set_written(10);
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