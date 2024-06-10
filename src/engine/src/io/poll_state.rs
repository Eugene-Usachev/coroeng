// TODO docs

use std::io::Error;
use std::fmt::{Debug, Formatter};
import_fd_for_os!();
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use crate::coroutine::coroutine::CoroutineImpl;
use crate::net::tcp::TcpStream;
use crate::buf::Buffer;
use crate::import_fd_for_os;

pub struct EmptyState {
    fd: RawFd
}

pub struct AcceptTcpState {
    pub(crate) fd: RawFd,
    pub(crate) coroutine: CoroutineImpl,
    pub(crate) result: *mut Result<TcpStream, Error>
}

pub struct ConnectTcpState {
    pub(crate) address: SockAddr,
    pub(crate) socket: Socket,
    pub(crate) coroutine: CoroutineImpl,
    pub(crate) result: *mut Result<TcpStream, Error>
}

pub struct PollTcpState {
    pub(crate) fd: RawFd,
    pub(crate) coroutine: CoroutineImpl,
    pub(crate) result: *mut Result<&'static [u8], Error>
}

pub struct ReadTcpState {
    pub(crate) fd: RawFd,
    pub(crate) buffer: Buffer,
    pub(crate) coroutine: CoroutineImpl,
    pub(crate) result: *mut Result<&'static [u8], Error>
}

pub struct WriteTcpState {
    pub(crate) fd: RawFd,
    pub(crate) buffer: Buffer,
    pub(crate) coroutine: CoroutineImpl,
    pub(crate) result: *mut Result<Option<Buffer>, Error>
}

pub struct WriteAllTcpState {
    pub(crate) fd: RawFd,
    pub(crate) buffer: Buffer,
    pub(crate) coroutine: CoroutineImpl,
    pub(crate) result: *mut Result<(), Error>
}

pub struct CloseTcpState {
    pub(crate) fd: RawFd,
    pub(crate) coroutine: CoroutineImpl
}


/// # Why using [`Box`]?
///
/// Typically, most states are [`PollTcpState`], which weighs 32 bytes (40 including the enum itself).
/// At this time, the heaviest states considering the enum itself weigh 80 bytes, which makes the entire enum [`State`] weigh 80 bytes.
/// To avoid this, states are stacked in a [`Box`], thereby allowing the enum itself to weigh 16 bytes (any state weighs 16 bytes + its own weight, except [`EmptyState`]).
/// This allows to reduce the overall weight of all states (in the example with [`PollTcpState`] State([`PollTcpState`]) now weighs 48 bytes).
/// Since states are only used in IO operations, which are much more expensive than dereferencing, there is no performance impact.
pub enum State {
    Empty(EmptyState),
    AcceptTcp(Box<AcceptTcpState>),
    ConnectTcp(Box<ConnectTcpState>),
    PollTcp(Box<PollTcpState>),
    ReadTcp(Box<ReadTcpState>),
    /// Tells the selector that [`WriteTcpState`](crate::io::WriteTcpState) or another writable [`State`] is ready.
    /// So, this method returns before the write syscall is done. The writing will be done in [`Selector::poll`].
    ///
    /// # Panics
    ///
    /// Will lead to panic if the [`State`] does not exist at the time of [`Selector::poll`].
    ///
    /// # Note
    ///
    /// This method will lead to one syscall. Only the part of the buffer will be written.
    /// The number of bytes written will be stored in the result variable.
    WriteTcp(Box<WriteTcpState>),
    /// Tells the selector that [`WriteAllTcpState`](crate::io::WriteAllTcpState) or another writable [`State`] is ready.
    /// So, this method returns before the write syscall is done. The writing will be done in [`Selector::poll`].
    ///
    /// # Panics
    ///
    /// Will lead to panic if the [`State`] does not exist at the time of [`Selector::poll`].
    ///
    /// # Note
    ///
    /// This method can lead to one or more syscalls.
    WriteAllTcp(Box<WriteAllTcpState>),
    CloseTcp(Box<CloseTcpState>)
}

impl State {
    #[inline(always)]
    pub fn fd(&self) -> RawFd {
        match self {
            State::Empty(state) => { state.fd }
            State::AcceptTcp(state) => { state.fd }
            State::PollTcp(state) => { state.fd }
            State::ReadTcp(state) => { state.fd }
            State::WriteTcp(state) => { state.fd }
            State::WriteAllTcp(state) => { state.fd }
            State::CloseTcp(state) => { state.fd }

            _ => { panic!("[BUG] tried to get fd from {self:?} token") }
        }
    }

    pub fn new_empty(fd: RawFd) -> Self {
        State::Empty(EmptyState { fd })
    }

    #[inline(always)]
    pub fn new_accept_tcp(listener: RawFd, coroutine: CoroutineImpl, result: *mut Result<TcpStream, Error>) -> Self {
        State::AcceptTcp(Box::new(AcceptTcpState { fd: listener, coroutine, result }))
    }

    #[inline(always)]
    pub fn new_connect_tcp(address: SockAddr, coroutine: CoroutineImpl, result: *mut Result<TcpStream, Error>) -> Result<Self, (Error, CoroutineImpl)> {
        let socket_ = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP));
        if socket_.is_err() {
            unsafe {
                return Err((socket_.unwrap_err_unchecked(), coroutine));
            }
        }

        unsafe {
            Ok(State::ConnectTcp(Box::new(ConnectTcpState { address, socket: socket_.unwrap_unchecked(), coroutine, result })))
        }
    }

    #[inline(always)]
    pub fn new_poll_tcp(stream: RawFd, coroutine: CoroutineImpl, result: *mut Result<&'_ [u8], Error>) -> Self {
        let result = unsafe { std::mem::transmute(result) };
        State::PollTcp(Box::new(PollTcpState { fd: stream, coroutine, result }))
    }

    #[inline(always)]
    pub fn new_read_tcp(stream: RawFd, buf: Buffer, coroutine: CoroutineImpl, result: *mut Result<&'_ [u8], Error>) -> Self {
        let result = unsafe { std::mem::transmute(result) };
        State::ReadTcp(Box::new(ReadTcpState { fd: stream, buffer: buf, coroutine, result }))
    }

    #[inline(always)]
    pub fn new_write_tcp(stream: RawFd, buf: Buffer, coroutine: CoroutineImpl, result: *mut Result<Option<Buffer>, Error>) -> Self {
        State::WriteTcp(Box::new(WriteTcpState { fd: stream, buffer: buf, coroutine, result }))
    }

    #[inline(always)]
    pub fn new_write_all_tcp(stream: RawFd, buf: Buffer, coroutine: CoroutineImpl, result: *mut Result<(), Error>) -> Self {
        State::WriteAllTcp(Box::new(WriteAllTcpState { fd: stream, buffer: buf, coroutine, result }))
    }

    #[inline(always)]
    pub fn new_close_tcp(stream: RawFd, coroutine: CoroutineImpl) -> Self {
        State::CloseTcp(Box::new(CloseTcpState { fd: stream, coroutine }))
    }
}

impl Debug for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            State::Empty(state) => { write!(f, "Empty, fd: {:?}", state.fd) }
            State::AcceptTcp(state) => { write!(f, "AcceptTcp, fd: {:?}", state.fd) }
            State::ConnectTcp(state) => {
                write!(
                    f,
                    "ConnectTcp, addr: {:?}",
                    state.address
                )
            }
            State::PollTcp(state) => { write!(f, "PollTcp, fd: {:?}", state.fd) }
            State::ReadTcp(state) => { write!(f, "ReadTcp, fd: {:?}", state.fd) }
            State::WriteTcp(state) => { write!(f, "WriteTcp, fd: {:?}", state.fd) }
            State::WriteAllTcp(state) => { write!(f, "WriteAllTcp, fd: {:?}", state.fd) }
            State::CloseTcp(state) => { write!(f, "CloseTcp, fd: {:?}", state.fd) }
        }
    }
}