//! This module contains [`TcpStream`].
use std::io::Error;
use std::os::fd::RawFd;
use crate::coroutine::{CoroutineImpl, YieldStatus};
use crate::io::State;
use crate::{local_scheduler};
use crate::utils::{Buffer, Ptr};

// TODO docs for connect. Here we can add reference to docs in TcpListener
/// A TCP stream between a local and a remote socket.
///
/// # Close
///
/// [`TcpStream`] is automatically closed when it is dropped.
///
/// # Examples
///
/// ```rust
/// use std::io::Error;
/// use engine::net::TcpStream;
/// use engine::{coro, spawn_local};
///
/// #[coro]
/// fn handle_tcp_client(mut stream: TcpStream) {
///     loop {
///         let slice: &[u8] = (yield stream.read()).unwrap();
///
///         if slice.is_empty() {
///             break;
///         }
///
///         let mut buf = engine::buf::buffer();
///         buf.append(slice);
///
///         let res: Result<(), Error> = yield TcpStream::write_all(&mut stream, buf);
///
///         if res.is_err() {
///             println!("write failed, reason: {}", res.err().unwrap());
///             break;
///         }
///     }
/// }
///
/// #[coro]
/// fn connect_to_server() -> Result<(), Error> {
///     let mut stream: TcpStream = yield TcpStream::connect("localhost:8081");
///
///     let msg = b"Hello, world!";
///     yield TcpStream::write_all(&mut stream, msg)?;
///
///     let response: &[u8] = (yield stream.read()).unwrap();
///     println!("Received: {:?}", response);
///
///     Ok(())
/// }
///
/// spawn_local!(connect_to_server());
/// ```
pub struct TcpStream {
    is_registered: bool,
    data: Ptr<State>
}

impl TcpStream {
    /// Create a new `TcpStream` from a raw file descriptor.
    pub fn new(fd: RawFd) -> Self {
        Self {
            is_registered: false,
            data: Ptr::new(State::new_empty(fd))
        }
    }

    /// Returns the state_ptr of the [`TcpStream`].
    ///
    /// Uses for low-level work with the scheduler. If you don't know what it is, don't use it.
    #[inline(always)]
    pub fn state_ptr(&mut self) -> Ptr<State> {
        self.data
    }

    /// Returns the is_registered for the [`TcpStream`].
    pub fn is_registered(&self) -> bool {
        self.is_registered
    }

    /// Sets the is_registered for the [`TcpStream`].
    pub fn set_registered(&mut self, is_registered: bool) {
        self.is_registered = is_registered;
    }

    /// Reads data from the stream. It will wait (non-blocking) until data is available or an error occurs.
    /// When a coroutine is woken up, returns a reference to a slice of read bytes or an error.
    ///
    /// The length of the slice is equal to the number of bytes read.
    ///
    /// # Note
    ///
    /// Returning a reference to a slice of bytes allows application to avoid copying and allocations in usual cases.
    ///
    /// # Warning
    ///
    /// After `yielding`, the returned slice may be rewritten. So, if you want to `yield` after read, you must copy the slice into a new one.
    ///
    /// # Example
    ///
    /// ```rust
    /// use engine::coro;
    /// use engine::net::TcpStream;
    /// use std::io::Error;
    /// use engine::utils::{buffer, Buffer};
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
    pub fn read(&mut self, res: *mut Result<&'static [u8], Error>) -> YieldStatus {
        let is_registered = self.is_registered();
        if !is_registered {
            self.set_registered(true);
        }
        YieldStatus::tcp_read(is_registered, self.data, res)
    }

    /// Writes data to the stream.
    /// Returns [`Option<Buffer>`] or an error.
    /// If Option is [`None`], all data has been written.
    /// Else `Buffer` contains new `offset` field, that indicates how many bytes have been written.
    ///
    /// # Note
    ///
    /// Don't worry about moving [`Buffer`]. In most cases buffer will be moved in [`BufPool`](crate::buf::BufPool).
    /// So using [`buffer`](crate::buf::buffer) and moving the buffer is lead to reusing memory and avoiding allocations.
    ///
    /// # Example
    ///
    /// ```rust
    /// use engine::coro;
    /// use engine::net::TcpStream;
    /// use engine::buf::Buffer;
    /// use std::io::Error;
    ///
    /// #[coro]
    /// fn write_to_stream(mut stream: TcpStream, mut buf: Buffer) {
    ///     loop {
    ///         let res: Result<Option<Buffer>, Error> = yield stream.write(buf);
    ///         if res.is_err() {
    ///             println!("write failed, reason: {}", res.err().unwrap());
    ///             break;
    ///         }
    ///
    ///         if let Some(new_buf) = res.unwrap() {
    ///             buf = new_buf;
    ///         } else {
    ///             break;
    ///         }
    ///     }
    /// }
    /// ```
    pub fn write(&mut self, buf: Buffer, res: *mut Result<Option<Buffer>, Error>) -> YieldStatus {
        YieldStatus::tcp_write(self.data, buf, res)
    }

    /// Writes all data to the stream or returns an error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use engine::coro;
    /// use engine::net::TcpStream;
    /// use engine::buf::Buffer;
    /// use std::io::Error;
    ///
    /// #[coro]
    /// fn write_to_stream(mut stream: TcpStream, mut buf: Buffer) {
    ///     let res: Result<(), Error> = yield stream.write_all(buf);
    ///     if res.is_err() {
    ///         println!("write failed, reason: {}", res.err().unwrap());
    ///         break;
    ///     }
    ///     // all data has been written
    /// }
    /// ```
    pub fn write_all(&mut self, buf: Buffer, res: *mut Result<(), Error>) -> YieldStatus {
        YieldStatus::tcp_write_all(self.data, buf, res)
    }

    /// Closes the stream.
    fn close(state_ref: Ptr<State>) -> YieldStatus {
        YieldStatus::tcp_close(state_ref)
    }
}

fn close_stream(state_ref: Ptr<State>) -> CoroutineImpl {
    Box::pin(#[coroutine] static move || {
        yield TcpStream::close(state_ref);
        unsafe { state_ref.drop_in_place(); }
    })
}

impl Drop for TcpStream {
    fn drop(&mut self) {
        let state_ptr = self.data;
        if self.is_registered() {
            local_scheduler().sched(close_stream(state_ptr));
        } else {
            unsafe { state_ptr.drop_in_place(); }
        }
    }
}