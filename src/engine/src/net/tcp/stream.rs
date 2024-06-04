//! This module contains [`TcpStream`].
use std::io::Error;
use std::net::SocketAddr;
use std::os::fd::RawFd;
use crate::coroutine::{CoroutineImpl, YieldStatus};
use crate::io::{AsyncRead, AsyncWrite, PollState};
use crate::{local_scheduler};
use crate::buf::Buffer;
use crate::utils::Ptr;

// TODO docs for connect. Here we can add reference to docs in TcpListener
/// A TCP stream between a local and a remote socket.
///
/// # Close
///
/// [`TcpStream`] is automatically closed when it is dropped.
///
/// # Examples
///
/// ```ignore
/// use std::io::Error;
/// use engine::net::TcpStream;
/// use engine::{coro, spawn_local};
/// use engine::io::{AsyncWrite, AsyncRead};
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
    data: Ptr<PollState>
}

impl TcpStream {
    /// Create a new `TcpStream` from a raw file descriptor.
    pub fn new(fd: RawFd) -> Self {
        Self {
            is_registered: false,
            data: Ptr::new(PollState::new_empty(fd))
        }
    }

    // TODO more docs
    /// Connects to the specified address.
    pub fn connect(addr: SocketAddr, res: *mut Result<TcpStream, Error>) -> YieldStatus {
        YieldStatus::tcp_connect(addr, res)
    }

    /// Returns the state_ptr of the [`TcpStream`].
    ///
    /// Uses for low-level work with the scheduler. If you don't know what it is, don't use it.
    #[inline(always)]
    pub fn state_ptr(&mut self) -> Ptr<PollState> {
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

    /// Closes the stream.
    fn close(state_ref: Ptr<PollState>) -> YieldStatus {
        YieldStatus::tcp_close(state_ref)
    }
}

impl AsyncRead<&'static [u8]> for TcpStream {
    #[inline(always)]
    fn read(&mut self, res: *mut Result<&'static [u8], Error>) -> YieldStatus {
        let is_registered = self.is_registered();
        // TODO think about it
        if !is_registered {
            self.set_registered(true);
        }
        YieldStatus::tcp_read(is_registered, self.data, res)
    }
}

impl AsyncWrite<Buffer> for TcpStream {
    #[inline(always)]
    fn write(&mut self, data: Buffer, res: *mut Result<Option<Buffer>, Error>) -> YieldStatus {
        YieldStatus::tcp_write(self.data, data, res)
    }

    #[inline(always)]
    fn write_all(&mut self, data: Buffer, res: *mut Result<(), Error>) -> YieldStatus {
        YieldStatus::tcp_write_all(self.data, data, res)
    }
}

fn close_stream(state_ref: Ptr<PollState>) -> CoroutineImpl {
    Box::pin(#[coroutine] static move || {
        yield TcpStream::close(state_ref);
        unsafe { state_ref.deallocate(); }
    })
}

impl Drop for TcpStream {
    fn drop(&mut self) {
        let state_ptr = self.data;
        if self.is_registered() {
            local_scheduler().sched(close_stream(state_ptr));
        } else {
            unsafe { state_ptr.deallocate(); }
        }
    }
}