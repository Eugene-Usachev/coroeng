use std::io::Error;
use std::net::{SocketAddr};
use std::os::fd::{IntoRawFd, RawFd};
use proc::coro;
use crate::coroutine::{CoroutineImpl, YieldStatus};
use crate::io::sys::unix::epoll::net::get_tcp_listener_fd;
use crate::net::tcp::TcpStream;
use crate::io::State;
use crate::{local_scheduler, spawn_local};
use crate::utils::Ptr;

/// A TCP socket server, listening for connections.
///
/// # Close
///
/// [`TcpListener`] is automatically closed when it is dropped.
///
/// # Examples
///
/// ```
/// use std::net::SocketAddr;
/// use engine::net::tcp::TcpListener;
/// use engine::io::io_yield;
/// use engine::{ret_yield, spawn_local};
/// use engine::net::TcpStream;
///
/// let mut listener = ret_yield!(TcpListener::new, "localhost:8081".to_socket_addrs().unwrap().next().unwrap());
/// loop {
///     let stream_ = ret_yield!(TcpListener::accept, &mut listener);
///     if stream_.is_err() {
///         println!("accept failed, reason: {}", stream_.err().unwrap());
///         continue;
///     }
///
///     let mut stream: TcpStream = stream_.unwrap();
///     spawn_local!({
///         loop {
///             let mut slice = ret_yield!(TcpStream::read, &mut stream).expect("read failed");
///             if slice.is_empty() {
///                 break;
///             }
///
///             ret_yield!(TcpStream::write_all, &mut stream, slice.to_vec()).expect("write failed");
///         }
///     });
/// }
/// ```
pub struct TcpListener {
    pub(crate) state_ptr: Ptr<State>,
    /// OwnedFd is required for Drop
    pub(crate) is_registered: bool
}

impl TcpListener {
    pub fn from_fd(fd: RawFd) -> Self {
        Self {
            state_ptr: Ptr::new(State::new_empty(fd)),
            is_registered: false
        }
    }

    /// Returns the state_id of the [`TcpListener`].
    ///
    /// Uses for low-level work with the scheduler. If you don't know what it is, don't use it.
    pub fn state_ptr(&mut self) -> Ptr<State> {
        self.state_ptr
    }

    /// Returns the fd for the [`TcpListener`].
    pub(crate) fn get_fd(addr: SocketAddr) -> RawFd {
        get_tcp_listener_fd(addr).into_raw_fd()
    }

    /// Creates a new TcpListener.
    ///
    /// # Arguments
    ///
    /// * `addr` - The SocketAddr to bind the TcpListener to.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use engine::ret_yield;
    /// use engine::net::tcp::Listener;
    /// use engine::io::io_yield;
    ///
    /// let mut listener = ret_yield!(TcpListener::new, "localhost:8081".to_socket_addrs().unwrap().next().unwrap());
    /// ```
    pub fn new(addr: SocketAddr, res: *mut TcpListener) -> YieldStatus {
        YieldStatus::new_tcp_listener(addr, res)
    }

    /// Accepts a new TcpStream.
    ///
    /// # Arguments
    ///
    /// * `TcpListener` - The [`TcpListener`] to accept connections from.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use engine::net::{TcpListener, TcpStream};
    /// use engine::io::io_yield;
    /// use engine::{ret_yield, spawn_local};
    ///
    /// let mut listener = ret_yield!(TcpListener::new, "localhost:8081".to_socket_addrs().unwrap().next().unwrap());
    /// loop {
    ///     let stream_ = ret_yield!(TcpListener::accept, &mut listener);
    ///     if stream_.is_err() {
    ///         println!("accept failed, reason: {}", stream_.err().unwrap());
    ///         continue;
    ///     }
    ///
    ///     let mut stream: TcpStream = stream_.unwrap();
    ///     spawn_local!({
    ///         loop {
    ///             let mut slice = ret_yield!(TcpStream::read, &mut stream).expect("read failed");
    ///             if slice.is_empty() {
    ///                 break;
    ///             }
    ///
    ///             ret_yield!(TcpStream::write_all, &mut stream, slice.to_vec()).expect("write failed");
    ///         }
    ///     });
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
    fn close(state_ref: Ptr<State>) -> YieldStatus {
        YieldStatus::tcp_close(state_ref)
    }
}

fn close_listener(state_ptr: Ptr<State>) -> CoroutineImpl {
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