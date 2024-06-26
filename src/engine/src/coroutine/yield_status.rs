//! This module contains a description of [`YieldStatus`] for low-level work with the scheduler.
//! Please use high-level functions for working with the scheduler if it is possible.

use std::net::SocketAddr;
use std::time::Duration;
use crate::io::PollState;
use crate::net::{TcpListener, TcpStream};
use crate::buf::{Buffer};
use crate::utils::Ptr;

/// Represents a new TCP listener to be created.
#[derive(Debug)]
pub struct NewTcpListener {
    /// The address on which the TCP listener will listen.
    pub(crate) address: SocketAddr,
    /// Pointer to store the newly created [`TcpListener`].
    pub(crate) listener_ptr: *mut TcpListener,
}

/// Represent a TCP connect operation.
#[derive(Debug)]
pub struct TcpConnect {
    /// The address on which the TCP listener will listen.
    pub(crate) address: SocketAddr,
    /// Pointer to store the newly created [`TcpStream`].
    pub(crate) stream_ptr: *mut Result<TcpStream, std::io::Error>,
}

/// Represents a TCP accept operation.
#[derive(Debug)]
pub struct TcpAccept {
    /// Indicates whether the socket is registered to the selector.
    pub(crate) is_registered: bool,
    /// The state ID associated with the TCP accept operation.
    pub(crate) state_ref: Ptr<PollState>,
    /// Pointer to store the result of the TCP accept operation.
    /// If success, the result will contain a [`TcpStream`].
    pub(crate) result_ptr: *mut Result<TcpStream, std::io::Error>,
}

/// Represents a TCP read operation.
#[derive(Debug)]
pub struct TcpRead {
    /// Indicates whether the socket is registered to the selector.
    pub(crate) is_registered: bool,
    /// The state ID associated with the TCP read operation.
    pub(crate) state_ref: Ptr<PollState>,
    /// Pointer to store the result of the TCP read operation.
    /// If success, the result will contain a slice of bytes read.
    pub(crate) result_ptr: *mut Result<&'static [u8], std::io::Error>,
}

/// Represents a TCP write operation.
#[derive(Debug)]
pub struct TcpWrite {
    /// The state ID associated with the TCP write operation.
    pub(crate) state_ref: Ptr<PollState>,
    /// The buffer containing data to be written.
    pub(crate) buffer: Buffer,
    /// Pointer to store the result of the TCP write operation.
    /// If success, the result will contain the number of bytes written.
    pub(crate) result_ptr: *mut Result<Option<Buffer>, std::io::Error>,
}

/// Represents a TCP write all operation.
#[derive(Debug)]
pub struct TcpWriteAll {
    /// The state ID associated with the TCP write all operation.
    pub(crate) state_ref: Ptr<PollState>,
    /// The buffer containing data to be written.
    pub(crate) buffer: Buffer,
    /// Pointer to store the result of the TCP write all operation.
    /// If success, the result will contain `()`.
    pub(crate) result_ptr: *mut Result<(), std::io::Error>,
}

/// Represents a TCP close operation.
#[derive(Debug)]
pub struct TcpClose {
    /// The state ID associated with the TCP close operation.
    pub(crate) state_ptr: Ptr<PollState>,
}

/// The status of the coroutine yield. This is the one way to communicate with the scheduler.
/// It uses instead of await for async programming, and uses for creating new coroutines and for let the scheduler wake other coroutines up.
#[derive(Debug)]
pub enum YieldStatus {
    /// [`Yield`] takes no arguments.
    ///
    /// If yielded, the coroutine let the scheduler wake other coroutines up.
    /// The current coroutine will be woken up by the scheduler after all other coroutines.
    Yield,

    /// [`End`] takes no arguments.
    ///
    /// If yielded, the [`scheduler`](crate::scheduler::Scheduler) will be terminated.
    ///
    /// # Be careful
    ///
    /// It means, that [`uninit`](crate::run::uninit) will be called.
    /// After this [`Selector`](crate::io::selector::Selector) will be dropped and all poll states will be leaked (with memory).
    ///
    /// # Do not call this function in a production!
    /// Because it may cause a memory leak and coroutine leak (which can lead to deadlocks). It is only intended for testing and should only be used for testing purposes.
    End,

    /// [`Sleep`] takes the duration.
    ///
    /// # Arguments
    ///
    /// * [`Duration`] - The duration to sleep.
    ///
    /// If yielded, the coroutine will sleep for at least the duration.
    Sleep(Duration),

    /// [`NewTcpListener`] takes the address and a pointer.
    ///
    /// If yielded, the new listener will be stored in the pointer.
    NewTcpListener(NewTcpListener),

    /// [`TcpConnect`] takes the address and a pointer.
    ///
    /// If yielded, the new connection will be stored in the pointer.
    TcpConnect(TcpConnect),

    /// [`TcpAccept`] takes is registered to the selector, a state id and a result pointer.
    ///
    /// If yielded, the connection will be accepted and [`TcpStream`] will be stored in the result pointer.
    TcpAccept(TcpAccept),

    /// [`TcpRead`] takes is registered to the selector, the state id, and a result pointer.
    ///
    /// If yielded, the connection assigned to this state will be read into the inner buffer.
    /// The read result will be stored in the result pointer.
    /// If successful, the slice reference will be stored in the result pointer.
    /// If the length of the slice is 0, the connection has been terminated by the other side.
    ///
    /// After next yield or return, the buffer will be rewritten.
    ///
    /// # Undefined behavior
    ///
    /// The undefined behavior will occur if the buffer will be used after the next yield or return.
    ///
    TcpRead(TcpRead),

    /// [`TcpWrite`] takes the state id, a buffer and a result pointer.
    ///
    /// If yielded, a part of the buffer will be written (with a single syscall) to the connection assigned to this state.
    /// The write result will be stored in the result pointer. It will store the number of bytes written to the buffer if successful.
    TcpWrite(TcpWrite),

    /// [`TcpWriteAll`] takes the state id, a buffer and a result pointer.
    ///
    /// If yielded, the buffer will be written whole (maybe with multiple syscalls) to the connection assigned to this state.
    /// The write result will be stored in the result pointer.
    TcpWriteAll(TcpWriteAll),

    /// [`TcpClose`] takes the state id.
    /// If yielded, the connection assigned to this state will be closed, and the state will be removed.
    TcpClose(TcpClose)
}

impl YieldStatus {
    /// Create a YieldStatus variant [`Yield`](YieldStatus::Yield).
    pub fn yield_now() -> Self {
        YieldStatus::Yield
    }

    /// Create a YieldStatus variant [`End`](YieldStatus::End).
    pub fn end() -> Self {
        YieldStatus::End
    }

    /// Create a YieldStatus variant [`Sleep`](YieldStatus::Sleep).
    pub fn sleep(duration: Duration) -> Self {
        YieldStatus::Sleep(duration)
    }

    /// Create a YieldStatus variant [`NewTcpListener`](YieldStatus::NewTcpListener).
    pub fn new_tcp_listener(address: SocketAddr, listener_ptr: *mut TcpListener) -> Self {
        YieldStatus::NewTcpListener(NewTcpListener { address, listener_ptr })
    }

    /// Create a YieldStatus variant [`TcpConnect`](YieldStatus::TcpConnect).
    pub fn tcp_connect(address: SocketAddr, result_ptr: *mut Result<TcpStream, std::io::Error>) -> Self {
        YieldStatus::TcpConnect(TcpConnect { address, stream_ptr: result_ptr })
    }

    /// Create a YieldStatus variant [`TcpAccept`](YieldStatus::TcpAccept).
    pub fn tcp_accept(is_registered: bool, state_ref: Ptr<PollState>, result_ptr: *mut Result<TcpStream, std::io::Error>) -> Self {
        YieldStatus::TcpAccept(TcpAccept { is_registered, state_ref, result_ptr })
    }

    /// Create a YieldStatus variant [`TcpRead`](YieldStatus::TcpRead).
    pub fn tcp_read(is_registered: bool, state_ref: Ptr<PollState>, result_ptr: *mut Result<&'static [u8], std::io::Error>) -> Self {
        YieldStatus::TcpRead(TcpRead { is_registered, state_ref, result_ptr })
    }

    /// Create a YieldStatus variant [`TcpWrite`](YieldStatus::TcpWrite).
    pub fn tcp_write(state_ref: Ptr<PollState>, buffer: Buffer, result_ptr: *mut Result<Option<Buffer>, std::io::Error>) -> Self {
        YieldStatus::TcpWrite(TcpWrite { state_ref, buffer, result_ptr })
    }

    /// Create a YieldStatus variant [`TcpWriteAll`](YieldStatus::TcpWriteAll).
    pub fn tcp_write_all(state_ref: Ptr<PollState>, buffer: Buffer, result_ptr: *mut Result<(), std::io::Error>) -> Self {
        YieldStatus::TcpWriteAll(TcpWriteAll { state_ref, buffer, result_ptr })
    }

    /// Create a YieldStatus variant [`TcpClose`](YieldStatus::TcpClose).
    pub fn tcp_close(state_ref: Ptr<PollState>) -> Self {
        YieldStatus::TcpClose(TcpClose { state_ptr: state_ref })
    }
}