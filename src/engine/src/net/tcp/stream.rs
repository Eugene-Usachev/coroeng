//! This module contains [`TcpStream`].
use std::intrinsics::unlikely;
use std::io::Error;
use std::mem::MaybeUninit;
use std::net::SocketAddr;
use crate::coroutine::{YieldStatus};
use crate::io::{AsyncRead, AsyncWrite, State};
use crate::{local_scheduler};
use crate::buf::Buffer;
use crate::utils::Ptr;

// TODO docs for connect. Here we can add reference to docs in TcpListener and close
/// A TCP stream between a local and a remote socket.
///
/// # Close
///
/// If you want to close the stream, you can use the [`TcpStream::close`] method.
/// Be careful, if the other side closes the stream, you need not call [`TcpStream::close`].
///
/// If [`TcpStream`] has been registered, you need call [`TcpStream::deregister`].
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
// TODO register/deregister
pub struct TcpStream {
    state_ptr: Ptr<State>
}

impl TcpStream {
    /// Create a new `TcpStream` from a raw file descriptor.
    pub fn new(state_ptr: Ptr<State>) -> Self {
        Self {
            state_ptr
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
    pub fn state_ptr(&mut self) -> Ptr<State> {
        self.state_ptr
    }

    // TODO docs
    /// Closes the stream.
    fn close(&mut self) {
        let state_ptr = self.state_ptr;
        local_scheduler().sched(Box::pin(#[coroutine] static move || {
            let mut res_ = MaybeUninit::uninit();
            yield YieldStatus::tcp_close(state_ptr, res_.as_mut_ptr());
            let res = unsafe { res_.assume_init() };

            if unlikely(res.is_err()) {
                println!("Cannot close TcpStream, reason: {:?}", unsafe { res.unwrap_err_unchecked() });
            }
            local_scheduler().put_state_ptr(state_ptr);
        }));
    }
}

impl AsyncRead<Buffer> for TcpStream {
    #[inline(always)]
    fn read(&mut self, res: *mut Result<Buffer, Error>) -> YieldStatus {
        YieldStatus::tcp_read(self.state_ptr, res)
    }
}

impl AsyncWrite<Buffer> for TcpStream {
    #[inline(always)]
    fn write(&mut self, data: Buffer, res: *mut Result<Option<Buffer>, Error>) -> YieldStatus {
        YieldStatus::tcp_write(self.state_ptr, data, res)
    }

    #[inline(always)]
    fn write_all(&mut self, data: Buffer, res: *mut Result<(), Error>) -> YieldStatus {
        YieldStatus::tcp_write_all(self.state_ptr, data, res)
    }
}

impl Drop for TcpStream {
    fn drop(&mut self) {
        self.close();
    }
}