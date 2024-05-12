use std::io::Error;
use std::net::{SocketAddr};
use std::os::fd::{IntoRawFd, RawFd};
use crate::engine::coroutine::YieldStatus;
use crate::engine::io::sys::unix::epoll::net::get_tcp_listener_fd;
use crate::engine::net::tcp::TcpStream;
use crate::{spawn_local_move};
use crate::engine::io::Token;
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
    pub(crate) token_ptr: Ptr<Token>,
    /// OwnedFd is required for Drop
    pub(crate) is_registered: bool
}

impl TcpListener {
    pub fn from_fd(fd: RawFd) -> Self {
        Self {
            token_ptr: Ptr::new(Token::new_empty(fd)),
            is_registered: false
        }
    }

    /// Returns the token_id of the [`TcpListener`].
    ///
    /// Uses for low-level work with the scheduler. If you don't know what it is, don't use it.
    pub fn token_ptr(&mut self) -> Ptr<Token> {
        self.token_ptr
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
    /// use crate::engine::net::tcp::Listener;
    /// use crate::engine::io::io_yield;
    ///
    /// let mut listener = io_yield!(TcpListener::new, "localhost:8081".to_socket_addrs().unwrap().next().unwrap());
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
        YieldStatus::tcp_accept(is_registered, self.token_ptr, res)
    }

    /// Closes the [`TcpListener`] by token_id. After closing, the [`TcpListener`] can not be used.
    fn close(token_ref: Ptr<Token>) -> YieldStatus {
        YieldStatus::tcp_close(token_ref)
    }
}

impl Drop for TcpListener {
    fn drop(&mut self) {
        let token_ptr = self.token_ptr;
        if self.is_registered {
            spawn_local_move!({
                yield TcpListener::close(token_ptr);
                unsafe { token_ptr.drop_in_place(); }
            });
        } else {
            unsafe { token_ptr.drop_in_place(); }
        }
    }
}