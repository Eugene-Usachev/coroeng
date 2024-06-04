use std::io::Error;
use crate::coroutine::YieldStatus;

/// The AsyncRead trait provides asynchronous read functionality for various types of data.
pub trait AsyncRead<T> {
    /// Reads data from this reader. It will wait (non-blocking) until data is available or an error occurs.
    /// When a coroutine is woken up, returns a reference to a slice of read bytes or an error.
    ///
    /// # Where returns a reference to a slice of bytes
    ///
    /// The length of the slice is equal to the number of bytes read.
    ///
    /// ### Note
    ///
    /// Returning a reference to a slice of bytes allows application to avoid copying and allocations in usual cases.
    ///
    /// ### Warning
    ///
    /// After `yielding`, the returned slice may be rewritten. So, if you want to `yield` after read, you must copy the slice into a new one.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use engine::coro;
    /// use engine::net::TcpStream;
    /// use std::io::Error;
    /// use engine::buf::{buffer, Buffer};
    /// use engine::io::{AsyncWrite, AsyncRead};
    ///
    /// #[coro]
    /// fn handle_tcp_client(mut stream: TcpStream) {
    ///     let slice: &[u8] = (yield stream.read()).unwrap();
    ///     if slice.is_empty() {
    ///         return;
    ///     }
    ///
    ///     let mut buf = buffer();
    ///     buf.append(slice);
    ///
    ///     let res: Result<(), Error> = yield TcpStream::write_all(&mut stream, buf);
    ///
    ///     if res.is_err() {
    ///         println!("write failed, reason: {}", res.err().unwrap());
    ///         return;
    ///     }
    /// }
    ///
    /// const MUST_READ: usize = 100;
    ///
    /// #[coro]
    /// fn handle_tcp_client_with_maybe_yield(mut stream: TcpStream) {
    ///     let mut full_buf: Option<Buffer> = None;
    ///     let mut read = 0;
    ///     loop {
    ///         let slice: &[u8] = (yield stream.read()).unwrap();
    ///         if slice.is_empty() {
    ///             break;
    ///         }
    ///
    ///         read += slice.len();
    ///         if read == MUST_READ {
    ///             if full_buf.is_none() {
    ///                 // work with slice. Fast case, because we don't need to copy it.
    ///             } else {
    ///                 // work with full_buf. Slow case, because we need to copy it.
    ///             }
    ///             return;
    ///         }
    ///         if full_buf.is_none() {
    ///             full_buf = Some(buffer());
    ///         }
    ///         full_buf.as_mut().unwrap().append(slice);
    ///     }
    /// }
    /// ```
    fn read(&mut self, res: *mut Result<T, Error>) -> YieldStatus;
}