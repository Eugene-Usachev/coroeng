use std::io::Error;
use std::net::SocketAddr;
use std::os::fd::{IntoRawFd, RawFd};
use crate::engine::coroutine::coroutine::YieldStatus;
use crate::engine::io::sys::unix::epoll::net::get_listener_fd;
use crate::engine::net::tcp::TcpStream;
use crate::{spawn_local, spawn_local_move};

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
/// use crate::engine::net::tcp::Listener;
/// use crate::engine::io::io_yield;
///
/// let mut listener = io_yield!(TcpListener::new, "localhost:8081".to_socket_addrs().unwrap().next().unwrap());
/// loop {
///     let stream_ = io_yield!(TcpListener::accept, &mut listener);
///     if stream_.is_err() {
///         println!("accept failed, reason: {}", stream_.err().unwrap());
///         continue;
///     }
///
///     let mut stream: TcpStream = stream_.unwrap();
///     spawn_local_move!({
///         loop {
///             let mut slice = io_yield!(TcpStream::read, &mut stream).expect("read failed");
///             if slice.is_empty() {
///                 break;
///             }
///
///             io_yield!(TcpStream::write_all, &mut stream, slice.to_vec()).expect("write failed");
///         }
///     });
/// }
/// ```
pub struct TcpListener {
    pub(crate) token_id: usize,
    /// OwnedFd is required for Drop
    pub(crate) fd: RawFd,
    pub(crate) is_registered: bool
}

impl TcpListener {
    pub(crate) fn get_fd(addr: SocketAddr) -> RawFd {
        get_listener_fd(addr).into_raw_fd()
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
    /// use crate::engine::net::tcp::Listener;
    /// use crate::engine::io::io_yield;
    ///
    /// let mut listener = io_yield!(TcpListener::new, "localhost:8081".to_socket_addrs().unwrap().next().unwrap());
    /// ```
    pub fn new(addr: SocketAddr, res: *mut TcpListener) -> YieldStatus {
        YieldStatus::NewTcpListener(addr, res)
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
    /// use crate::engine::net::tcp::Listener;
    /// use crate::engine::io::io_yield;
    ///
    /// let mut listener = io_yield!(TcpListener::new, "localhost:8081".to_socket_addrs().unwrap().next().unwrap());
    /// loop {
    ///     let stream_ = io_yield!(TcpListener::accept, &mut listener);
    ///     if stream_.is_err() {
    ///         println!("accept failed, reason: {}", stream_.err().unwrap());
    ///         continue;
    ///     }
    ///
    ///     let mut stream: TcpStream = stream_.unwrap();
    ///     spawn_local_move!({
    ///         loop {
    ///             let mut slice = io_yield!(TcpStream::read, &mut stream).expect("read failed");
    ///             if slice.is_empty() {
    ///                 break;
    ///             }
    ///
    ///             io_yield!(TcpStream::write_all, &mut stream, slice.to_vec()).expect("write failed");
    ///         }
    ///     });
    /// }
    /// ```
    pub fn accept(&mut self, res: *mut Result<TcpStream, Error>) -> YieldStatus {
        let is_registered = self.is_registered;
        if !is_registered {
            self.is_registered = true;
        }
        YieldStatus::TcpAccept(is_registered, self.token_id, res)
    }

    /// Closes the [`TcpListener`] by token_id. After closing, the [`TcpListener`] can not be used.
    fn close(token_id: usize) -> YieldStatus {
        YieldStatus::TcpClose(token_id)
    }
}

impl Drop for TcpListener {
    fn drop(&mut self) {
        if !self.is_registered {
            return;
        }

        let token_id = self.token_id;
        spawn_local_move!({
            yield TcpListener::close(token_id);
        });
    }
}