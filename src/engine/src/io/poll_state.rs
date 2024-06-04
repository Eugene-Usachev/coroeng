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
/// At this time, the heaviest states considering the enum itself weigh 80 bytes, which makes the entire enum [`PollState`] weigh 80 bytes.
/// To avoid this, states are stacked in a [`Box`], thereby allowing the enum itself to weigh 16 bytes (any state weighs 16 bytes + its own weight, except [`EmptyState`]).
/// This allows to reduce the overall weight of all states (in the example with [`PollTcpState`] State([`PollTcpState`]) now weighs 48 bytes).
/// Since states are only used in IO operations, which are much more expensive than dereferencing, there is no performance impact.
pub enum PollState {
    Empty(EmptyState),
    AcceptTcp(Box<AcceptTcpState>),
    ConnectTcp(Box<ConnectTcpState>),
    PollTcp(Box<PollTcpState>),
    ReadTcp(Box<ReadTcpState>),
    WriteTcp(Box<WriteTcpState>),
    WriteAllTcp(Box<WriteAllTcpState>),
    CloseTcp(Box<CloseTcpState>)
}

impl PollState {
    #[inline(always)]
    pub fn fd(&self) -> RawFd {
        match self {
            PollState::Empty(state) => { state.fd }
            PollState::AcceptTcp(state) => { state.fd }
            PollState::PollTcp(state) => { state.fd }
            PollState::ReadTcp(state) => { state.fd }
            PollState::WriteTcp(state) => { state.fd }
            PollState::WriteAllTcp(state) => { state.fd }
            PollState::CloseTcp(state) => { state.fd }

            _ => { panic!("[BUG] tried to get fd from {self:?} token") }
        }
    }

    pub fn new_empty(fd: RawFd) -> Self {
        PollState::Empty(EmptyState { fd })
    }

    #[inline(always)]
    pub fn new_accept_tcp(listener: RawFd, coroutine: CoroutineImpl, result: *mut Result<TcpStream, Error>) -> Self {
        PollState::AcceptTcp(Box::new(AcceptTcpState { fd: listener, coroutine, result }))
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
            Ok(PollState::ConnectTcp(Box::new(ConnectTcpState { address, socket: socket_.unwrap_unchecked(), coroutine, result })))
        }
    }

    #[inline(always)]
    pub fn new_poll_tcp(stream: RawFd, coroutine: CoroutineImpl, result: *mut Result<&'_ [u8], Error>) -> Self {
        let result = unsafe { std::mem::transmute(result) };
        PollState::PollTcp(Box::new(PollTcpState { fd: stream, coroutine, result }))
    }

    #[inline(always)]
    pub fn new_read_tcp(stream: RawFd, buf: Buffer, coroutine: CoroutineImpl, result: *mut Result<&'_ [u8], Error>) -> Self {
        let result = unsafe { std::mem::transmute(result) };
        PollState::ReadTcp(Box::new(ReadTcpState { fd: stream, buffer: buf, coroutine, result }))
    }

    #[inline(always)]
    pub fn new_write_tcp(stream: RawFd, buf: Buffer, coroutine: CoroutineImpl, result: *mut Result<Option<Buffer>, Error>) -> Self {
        PollState::WriteTcp(Box::new(WriteTcpState { fd: stream, buffer: buf, coroutine, result }))
    }

    #[inline(always)]
    pub fn new_write_all_tcp(stream: RawFd, buf: Buffer, coroutine: CoroutineImpl, result: *mut Result<(), Error>) -> Self {
        PollState::WriteAllTcp(Box::new(WriteAllTcpState { fd: stream, buffer: buf, coroutine, result }))
    }

    #[inline(always)]
    pub fn new_close_tcp(stream: RawFd, coroutine: CoroutineImpl) -> Self {
        PollState::CloseTcp(Box::new(CloseTcpState { fd: stream, coroutine }))
    }
}

impl Debug for PollState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PollState::Empty(state) => { write!(f, "Empty, fd: {:?}", state.fd) }
            PollState::AcceptTcp(state) => { write!(f, "AcceptTcp, fd: {:?}", state.fd) }
            PollState::ConnectTcp(state) => {
                write!(
                    f,
                    "ConnectTcp, addr: {:?}",
                    state.address
                )
            }
            PollState::PollTcp(state) => { write!(f, "PollTcp, fd: {:?}", state.fd) }
            PollState::ReadTcp(state) => { write!(f, "ReadTcp, fd: {:?}", state.fd) }
            PollState::WriteTcp(state) => { write!(f, "WriteTcp, fd: {:?}", state.fd) }
            PollState::WriteAllTcp(state) => { write!(f, "WriteAllTcp, fd: {:?}", state.fd) }
            PollState::CloseTcp(state) => { write!(f, "CloseTcp, fd: {:?}", state.fd) }
        }
    }
}