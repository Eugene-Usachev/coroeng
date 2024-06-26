//! This module contains [`TcpListener`].
use std::intrinsics::unlikely;
use std::io::Error;
use std::mem::MaybeUninit;
use std::net::{SocketAddr};
use std::os::fd::{IntoRawFd, RawFd};
use crate::coroutine::{YieldStatus};
use crate::io::sys::unix::net::get_tcp_listener_fd;
use crate::net::tcp::TcpStream;
use crate::io::State;
use crate::{local_scheduler};
use crate::utils::Ptr;

// TODO new docs, because &[u8] and close
/// A TCP socket server, listening for connections.
///
/// # Close
///
/// [`TcpListener`] is automatically closed when it is dropped.
///
/// # Examples
///
/// ```ignore
/// use std::io::Error;
/// use std::net::{SocketAddr, ToSocketAddrs};
/// use engine::net::tcp::TcpListener;
/// use engine::{coro, spawn_local};
/// use engine::net::TcpStream;
/// use engine::io::{AsyncWrite, AsyncRead};
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
    pub(crate) state_ptr: Ptr<State>
}

impl TcpListener {
    /// Creates a new [`TcpListener`] from an existing fd.
    pub fn from_state_ptr(state_ptr: Ptr<State>) -> Self {
        Self {
            state_ptr
        }
    }

    /// Returns the state_ptr of the [`TcpListener`].
    ///
    /// Uses for low-level work with the scheduler. If you don't know what it is, don't use it.
    pub fn state_ptr(&mut self) -> Ptr<State> {
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
    /// ```ignore
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
    /// ```ignore
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
        YieldStatus::tcp_accept(self.state_ptr, res)
    }

    /// Closes the [`TcpListener`] by [`state_ptr`](#field.state_ptr). After closing, the [`TcpListener`] can not be used.
    fn close(state_ptr: Ptr<State>) {
        local_scheduler().sched(Box::pin(#[coroutine] static move || {
            let mut res_ = MaybeUninit::uninit();
            yield YieldStatus::close(state_ptr, res_.as_mut_ptr());
            let res = unsafe { res_.assume_init() };
            
            if unlikely(res.is_err()) {
                println!("Can't close TcpListener, reason: {:?}", unsafe { res.unwrap_err_unchecked() });
            }
            
            local_scheduler().put_state_ptr(state_ptr);
        }));
    }
}

impl Drop for TcpListener {
    fn drop(&mut self) {
        let state_ptr = self.state_ptr;
        TcpListener::close(state_ptr);
    }
}

