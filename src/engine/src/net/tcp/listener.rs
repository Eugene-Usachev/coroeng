//! This module contains [`TcpListener`].
use std::io::Error;
use std::net::{SocketAddr};
use std::os::fd::{IntoRawFd, RawFd};
use crate::coroutine::{CoroutineImpl, YieldStatus};
use crate::io::sys::unix::epoll::net::get_tcp_listener_fd;
use crate::net::tcp::TcpStream;
use crate::io::PollState;
use crate::{local_scheduler};
use crate::utils::Ptr;

/// A TCP socket server, listening for connections.
///
/// # Close
///
/// [`TcpListener`] is automatically closed when it is dropped.
///
/// # Examples
///
/// ```rust
/// use std::io::Error;
/// use std::net::{SocketAddr, ToSocketAddrs};
/// use engine::net::tcp::TcpListener;
/// use engine::{coro, spawn_local};
/// use engine::net::TcpStream;
/// use engine::io::AsyncWrite;
///
/// #[coro]
/// fn start_server() {
///     #[coro]
///     fn handle_tcp_client(mut stream: TcpStream) {
///         loop {
///             let slice: &[u8] = (yield stream.read()).unwrap();
///
///             if slice.is_empty() {
///                 break;
///             }
///
///             let mut buf = engine::buf::buffer();
///             buf.append(slice);
///
///             let res: Result<(), Error> = yield TcpStream::write_all(&mut stream, buf);
///
///             if res.is_err() {
///                 println!("write failed, reason: {}", res.err().unwrap());
///                 break;
///             }
///         }
///     }
///
///     let mut listener: TcpListener = yield TcpListener::new("localhost:8081".to_socket_addrs().unwrap().next().unwrap());
///     loop {
///         let stream_: Result<TcpStream, Error> = yield listener.accept();
///         if stream_.is_err() {
///             println!("accept failed, reason: {}", stream_.err().unwrap());
///             continue;
///         }
///
///         let stream = stream_.unwrap();
///         spawn_local!(handle_tcp_client(stream));
///     }
/// }
/// ```
pub struct TcpListener {
    pub(crate) state_ptr: Ptr<PollState>,
    /// OwnedFd is required for Drop
    pub(crate) is_registered: bool
}

impl TcpListener {
    /// Creates a new TcpListener from an existing fd.
    pub fn from_fd(fd: RawFd) -> Self {
        Self {
            state_ptr: Ptr::new(PollState::new_empty(fd)),
            is_registered: false
        }
    }

    /// Returns the state_ptr of the [`TcpListener`].
    ///
    /// Uses for low-level work with the scheduler. If you don't know what it is, don't use it.
    pub fn state_ptr(&mut self) -> Ptr<PollState> {
        self.state_ptr
    }

    // TODO remove pub
    /// Returns the fd for the [`TcpListener`].
    pub fn get_fd(addr: SocketAddr) -> RawFd {
        get_tcp_listener_fd(addr).into_raw_fd()
    }

    /// Creates a new TcpListener.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use engine::coro;
    /// use std::net::{SocketAddr, ToSocketAddrs};
    /// use engine::net::tcp::TcpListener;
    ///
    /// #[coro]
    /// fn new_listener() {
    ///     let mut listener: TcpListener = yield TcpListener::new("localhost:8081".to_socket_addrs().unwrap().next().unwrap());
    ///     yield listener.accept();
    /// }
    /// ```
    pub fn new(addr: SocketAddr, res: *mut TcpListener) -> YieldStatus {
        YieldStatus::new_tcp_listener(addr, res)
    }

    /// Accepts a new TcpStream.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::io::Error;
    /// use std::net::{SocketAddr, ToSocketAddrs};
    /// use engine::coro;
    /// use engine::net::tcp::TcpListener;
    /// use engine::net::TcpStream;
    ///
    /// #[coro]
    /// fn accept() {
    ///     let mut listener: TcpListener = yield TcpListener::new("localhost:8081".to_socket_addrs().unwrap().next().unwrap());
    ///     let stream_: Result<TcpStream, Error> = yield listener.accept();
    ///     if stream_.is_err() {
    ///         println!("accept failed, reason: {}", stream_.err().unwrap());
    ///         return;
    ///     }
    ///     let mut stream = stream_.unwrap();
    ///     yield stream.read();
    /// }
    /// ```
    pub fn accept(&mut self, res: *mut Result<TcpStream, Error>) -> YieldStatus {
        let is_registered = self.is_registered;
        if !is_registered {
            self.is_registered = true;
        }
        YieldStatus::tcp_accept(is_registered, self.state_ptr, res)
    }

    /// Closes the [`TcpListener`] by state_id. After closing, the [`TcpListener`] can not be used.
    fn close(state_ref: Ptr<PollState>) -> YieldStatus {
        YieldStatus::tcp_close(state_ref)
    }
}

fn close_listener(state_ptr: Ptr<PollState>) -> CoroutineImpl {
    Box::pin(#[coroutine] static move || {
        yield TcpListener::close(state_ptr);
        unsafe { state_ptr.drop_in_place(); }
    })
}

impl Drop for TcpListener {
    fn drop(&mut self) {
        let state_ptr = self.state_ptr;
        if self.is_registered {
            local_scheduler().sched(close_listener(state_ptr));
        } else {
            unsafe { state_ptr.drop_in_place(); }
        }
    }
}