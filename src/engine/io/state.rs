use std::io::Error;
use std::fmt::{Debug, Formatter};
use std::os::fd::{RawFd};
use crate::engine::coroutine::coroutine::CoroutineImpl;
use crate::engine::net::tcp::TcpStream;
use crate::engine::utils::Buffer;

pub(crate) struct EmptyState {
    fd: RawFd
}

pub(crate) struct AcceptTcpState {
    pub(crate) fd: RawFd,
    pub(crate) coroutine: CoroutineImpl,
    pub(crate) result: *mut Result<TcpStream, Error>
}

pub(crate) struct PollTcpState {
    pub(crate) fd: RawFd,
    pub(crate) coroutine: CoroutineImpl,
    pub(crate) result: *mut Result<&'static [u8], Error>
}

pub(crate) struct ReadTcpState {
    pub(crate) fd: RawFd,
    pub(crate) buffer: Buffer,
    pub(crate) coroutine: CoroutineImpl,
    pub(crate) result: *mut Result<&'static [u8], Error>
}

pub(crate) struct WriteTcpState {
    pub(crate) fd: RawFd,
    pub(crate) buffer: Buffer,
    pub(crate) coroutine: CoroutineImpl,
    pub(crate) result: *mut Result<usize, Error>
}

pub(crate) struct WriteAllTcpState {
    pub(crate) fd: RawFd,
    pub(crate) buffer: Buffer,
    pub(crate) coroutine: CoroutineImpl,
    pub(crate) bytes_written: usize,
    pub(crate) result: *mut Result<(), Error>
}

pub(crate) struct CloseTcpState {
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
pub(crate) enum State {
    Empty(EmptyState),
    AcceptTcp(Box<AcceptTcpState>),
    PollTcp(Box<PollTcpState>),
    ReadTcp(Box<ReadTcpState>),
    WriteTcp(Box<WriteTcpState>),
    WriteAllTcp(Box<WriteAllTcpState>),
    CloseTcp(Box<CloseTcpState>)
}

impl State {
    #[inline(always)]
    pub(crate) fn fd(&self) -> RawFd {
        match self {
            State::Empty(state) => { state.fd }
            State::AcceptTcp(state) => { state.fd }
            State::PollTcp(state) => { state.fd }
            State::ReadTcp(state) => { state.fd }
            State::WriteTcp(state) => { state.fd }
            State::WriteAllTcp(state) => { state.fd }
            State::CloseTcp(state) => { state.fd }
        }
    }

    pub(crate) fn new_empty(fd: RawFd) -> Self {
        State::Empty(EmptyState { fd })
    }

    #[inline(always)]
    pub(crate) fn new_accept_tcp(listener: RawFd, coroutine: CoroutineImpl, result: *mut Result<TcpStream, Error>) -> Self {
        State::AcceptTcp(Box::new(AcceptTcpState { fd: listener, coroutine, result }))
    }

    #[inline(always)]
    pub(crate) fn new_poll_tcp(stream: RawFd, coroutine: CoroutineImpl, result: *mut Result<&'_ [u8], Error>) -> Self {
        let result = unsafe { std::mem::transmute(result) };
        State::PollTcp(Box::new(PollTcpState { fd: stream, coroutine, result }))
    }

    #[inline(always)]
    pub(crate) fn new_read_tcp(stream: RawFd, buf: Buffer, coroutine: CoroutineImpl, result: *mut Result<&'_ [u8], Error>) -> Self {
        let result = unsafe { std::mem::transmute(result) };
        State::ReadTcp(Box::new(ReadTcpState { fd: stream, buffer: buf, coroutine, result }))
    }

    #[inline(always)]
    pub(crate) fn new_write_tcp(stream: RawFd, buf: Buffer, coroutine: CoroutineImpl, result: *mut Result<usize, Error>) -> Self {
        State::WriteTcp(Box::new(WriteTcpState { fd: stream, buffer: buf, coroutine, result }))
    }

    #[inline(always)]
    pub(crate) fn new_write_all_tcp(stream: RawFd, buf: Buffer, coroutine: CoroutineImpl, result: *mut Result<(), Error>) -> Self {
        State::WriteAllTcp(Box::new(WriteAllTcpState { fd: stream, buffer: buf, coroutine, bytes_written: 0, result }))
    }

    #[inline(always)]
    pub(crate) fn new_close_tcp(stream: RawFd, coroutine: CoroutineImpl) -> Self {
        State::CloseTcp(Box::new(CloseTcpState { fd: stream, coroutine }))
    }
}

impl Debug for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            State::Empty(state) => { write!(f, "Empty, fd: {}", state.fd) }
            State::AcceptTcp(state) => { write!(f, "AcceptTcp, fd: {}", state.fd) }
            State::PollTcp(state) => { write!(f, "PollTcp, fd: {}", state.fd) }
            State::ReadTcp(state) => { write!(f, "ReadTcp, fd: {}", state.fd) }
            State::WriteTcp(state) => { write!(f, "WriteTcp, fd: {}", state.fd) }
            State::WriteAllTcp(state) => { write!(f, "WriteAllTcp, fd: {}", state.fd) }
            State::CloseTcp(state) => { write!(f, "CloseTcp, fd: {}", state.fd) }
        }
    }
}