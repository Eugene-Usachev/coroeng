//! This module contains a description of [`Coroutine`], [`YieldStatus`], [`CoroutineImpl`] for working with the scheduler.
//! You can use this module for low-level work with the scheduler.

use std::net::SocketAddr;
use std::pin::Pin;
use std::ops::Coroutine as StdCoroutine;
use std::time::Duration;

use crate::engine::net::tcp::TcpStream;
use crate::engine::net::tcp::TcpListener;
use crate::utils::Buffer;

/// The status of the coroutine yield. This is the one way to communicate with the scheduler.
/// It uses instead of await for async programming, and uses for creating new coroutines and for let the scheduler wake other coroutines up.
#[derive(Debug)]
pub enum YieldStatus {
    /// [`Yield`] takes no arguments.
    ///
    /// If yielded, the coroutine let the scheduler wake other coroutines up.
    /// The current coroutine will be woken up by the scheduler after all other coroutines.
    Yield,
    /// [`Sleep`] takes the duration.
    ///
    /// If yielded, the coroutine will sleep for at least the duration.
    Sleep(Duration),

    /// [`NewTcpListener`] takes the address and a pointer.
    ///
    /// If yielded, the new listener will be stored in the pointer.
    NewTcpListener(SocketAddr, *mut TcpListener),
    /// [`TcpAccept`] takes is registered to the selector, a token id and a result pointer.
    ///
    /// If yielded, the connection will be accepted and [`TcpStream`] will be stored in the result pointer.
    TcpAccept(bool, usize, *mut Result<TcpStream, std::io::Error>),
    /// [`TcpRead`] takes is registered to the selector, the token id, and a result pointer.
    ///
    /// If yielded, the connection assigned to this token will be read into the inner buffer.
    /// The read result will be stored in the result pointer.
    /// If successful, the slice reference will be stored in the result pointer.
    /// If the length of the slice is 0, the connection has been terminated by the other side.
    ///
    /// After next yield or return, the buffer will be rewritten.
    ///
    /// # Undefined behavior
    ///
    /// The undefined behavior will occur if the buffer will be used after the next yield or return.
    TcpRead(bool, usize, *mut Result<&'static [u8], std::io::Error>),
    /// [`TcpWrite`] takes the token id, a buffer and a result pointer.
    ///
    /// If yielded, a part of the buffer will be written (with a single syscall) to the connection assigned to this token.
    /// The write result will be stored in the result pointer. It will store the number of bytes written to the buffer if successful.
    TcpWrite(usize, Buffer, *mut Result<usize, std::io::Error>),
    /// [`TcpWriteAll`] takes the token id, a buffer and a result pointer.
    ///
    /// If yielded, the buffer will be written whole (maybe with multiple syscalls) to the connection assigned to this token.
    /// The write result will be stored in the result pointer.
    TcpWriteAll(usize, Buffer, *mut Result<(), std::io::Error>),
    /// [`TcpClose`] takes the token id.
    /// If yielded, the connection assigned to this token will be closed, and the token will be removed.
    TcpClose(usize),

    /// [`Never`] calls panic if yielded.
    /// This is to tell Rust that the closure is a coroutine.
    Never
}

/// The alias for [`StdCoroutine`]<Yield=YieldStatus, Return=()>.
/// The scheduler works only with this type of the coroutines.
pub trait Coroutine = StdCoroutine<Yield=YieldStatus, Return=()>;

/// The alias for [`Pin`]<[`Box`]<dyn [`Coroutine`]>>.
/// The scheduler works only with this type of the coroutines.
pub type CoroutineImpl = Pin<Box<dyn Coroutine>>;
