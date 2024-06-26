// TODO update docs
use std::io::Error;
use crate::buf::Buffer;
use crate::coroutine::YieldStatus;

/// The AsyncRead trait provides asynchronous read.
///
/// # Position based read
///
/// [`AsyncPRead`] is a position based read.
pub trait AsyncRead {
    /// Reads data from this reader. It will wait (non-blocking) until data is available or an error occurs.
    /// When a coroutine is woken up, returns [`Buffer`] with `len` field where the number of bytes read is stored.
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
    ///     let buf: Buffer = (yield stream.read()).unwrap();
    ///     if buf.is_empty() {
    ///         return;
    ///     }
    ///
    ///     let res: Result<(), Error> = yield TcpStream::write_all(&mut stream, buf);
    ///
    ///     if res.is_err() {
    ///         println!("write failed, reason: {}", res.err().unwrap());
    ///         return;
    ///     }
    /// }
    ///
    /// // TODO docs. Code below is not working. Maybe write ReadIntoBuf?
    /// 
    /// const MUST_READ: usize = 100;
    ///
    /// #[coro]
    /// fn handle_tcp_client_with_maybe_yield(mut stream: TcpStream) {
    ///     let mut full_buf: Option<Buffer> = None;
    ///     let mut read = 0;
    ///     loop {
    ///         let buf: Buffer = (yield stream.read()).unwrap();
    ///         if buf.is_empty() {
    ///             break;
    ///         }
    ///
    ///         if buf.len() == MUST_READ {
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
    fn read(&mut self, res: *mut Result<Buffer, Error>) -> YieldStatus;
}

// TODO docs
pub trait AsyncPRead {
    fn pread(&mut self, offset: usize, res: *mut Result<Buffer, Error>) -> YieldStatus;
}