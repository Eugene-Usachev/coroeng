use std::io::Error;
use std::fmt::{Debug, Formatter};
use std::os::fd::{RawFd};
use crate::engine::coroutine::coroutine::CoroutineImpl;
use crate::engine::net::tcp::TcpStream;
use crate::utils::Buffer;

pub(crate) struct EmptyToken {
    fd: RawFd
}

pub(crate) struct AcceptTcpToken {
    pub(crate) fd: RawFd,
    pub(crate) coroutine: CoroutineImpl,
    pub(crate) result: *mut Result<TcpStream, Error>
}

pub(crate) struct PollTcpToken {
    pub(crate) fd: RawFd,
    pub(crate) coroutine: CoroutineImpl,
    pub(crate) result: *mut Result<&'static [u8], Error>
}

pub(crate) struct ReadTcpToken {
    pub(crate) fd: RawFd,
    pub(crate) buffer: Buffer,
    pub(crate) coroutine: CoroutineImpl,
    pub(crate) result: *mut Result<&'static [u8], Error>
}

pub(crate) struct WriteTcpToken {
    pub(crate) fd: RawFd,
    pub(crate) buffer: Buffer,
    pub(crate) coroutine: CoroutineImpl,
    pub(crate) result: *mut Result<usize, Error>
}

pub(crate) struct WriteAllTcpToken {
    pub(crate) fd: RawFd,
    pub(crate) buffer: Buffer,
    pub(crate) coroutine: CoroutineImpl,
    pub(crate) bytes_written: usize,
    pub(crate) result: *mut Result<(), Error>
}

pub(crate) struct CloseTcpToken {
    pub(crate) fd: RawFd,
    pub(crate) coroutine: CoroutineImpl
}

/// # Why using [`Box`]?
///
/// Typically, most tokens are [`PollTcpToken`], which weighs 32 bytes (40 including the enum itself).
/// At this time, the heaviest tokens considering the enum itself weigh 80 bytes, which makes the entire enum [`Token`] weigh 80 bytes.
/// To avoid this, tokens are stacked in a [`Box`], thereby allowing the enum itself to weigh 16 bytes (any token weighs 16 bytes + its own weight, except [`EmptyToken`]).
/// This allows to reduce the overall weight of all tokens (in the example with [`PollTcpToken`] Token([`PollTcpToken`]) now weighs 48 bytes).
/// Since tokens are only used in IO operations, which are much more expensive than dereferencing, there is no performance impact.
pub(crate) enum Token {
    Empty(EmptyToken),
    AcceptTcp(Box<AcceptTcpToken>),
    PollTcp(Box<PollTcpToken>),
    ReadTcp(Box<ReadTcpToken>),
    WriteTcp(Box<WriteTcpToken>),
    WriteAllTcp(Box<WriteAllTcpToken>),
    CloseTcp(Box<CloseTcpToken>)
}

impl Token {
    #[inline(always)]
    pub(crate) fn fd(&self) -> RawFd {
        match self {
            Token::Empty(token) => { token.fd }
            Token::AcceptTcp(token) => { token.fd }
            Token::PollTcp(token) => { token.fd }
            Token::ReadTcp(token) => { token.fd }
            Token::WriteTcp(token) => { token.fd }
            Token::WriteAllTcp(token) => { token.fd }
            Token::CloseTcp(token) => { token.fd }
        }
    }

    pub(crate) fn new_empty(fd: RawFd) -> Self {
        Token::Empty(EmptyToken { fd })
    }

    pub(crate) fn new_accept_tcp(listener: RawFd, coroutine: CoroutineImpl, result: *mut Result<TcpStream, Error>) -> Self {
        Token::AcceptTcp(Box::new(AcceptTcpToken { fd: listener, coroutine, result }))
    }

    pub(crate) fn new_poll_tcp(stream: RawFd, coroutine: CoroutineImpl, result: *mut Result<&'_ [u8], Error>) -> Self {
        let result = unsafe { std::mem::transmute(result) };
        Token::PollTcp(Box::new(PollTcpToken { fd: stream, coroutine, result }))
    }

    pub(crate) fn new_read_tcp(stream: RawFd, buf: Buffer, coroutine: CoroutineImpl, result: *mut Result<&'_ [u8], Error>) -> Self {
        let result = unsafe { std::mem::transmute(result) };
        Token::ReadTcp(Box::new(ReadTcpToken { fd: stream, buffer: buf, coroutine, result }))
    }

    pub(crate) fn new_write_tcp(stream: RawFd, buf: Buffer, coroutine: CoroutineImpl, result: *mut Result<usize, Error>) -> Self {
        Token::WriteTcp(Box::new(WriteTcpToken { fd: stream, buffer: buf, coroutine, result }))
    }

    pub(crate) fn new_write_all_tcp(stream: RawFd, buf: Buffer, coroutine: CoroutineImpl, result: *mut Result<(), Error>) -> Self {
        Token::WriteAllTcp(Box::new(WriteAllTcpToken { fd: stream, buffer: buf, coroutine, bytes_written: 0, result }))
    }

    pub(crate) fn new_close_tcp(stream: RawFd, coroutine: CoroutineImpl) -> Self {
        Token::CloseTcp(Box::new(CloseTcpToken { fd: stream, coroutine }))
    }
}

impl Debug for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Empty(token) => { write!(f, "Empty, fd: {}", token.fd) }
            Token::AcceptTcp(token) => { write!(f, "AcceptTcp, fd: {}", token.fd) }
            Token::PollTcp(token) => { write!(f, "PollTcp, fd: {}", token.fd) }
            Token::ReadTcp(token) => { write!(f, "ReadTcp, fd: {}", token.fd) }
            Token::WriteTcp(token) => { write!(f, "WriteTcp, fd: {}", token.fd) }
            Token::WriteAllTcp(token) => { write!(f, "WriteAllTcp, fd: {}", token.fd) }
            Token::CloseTcp(token) => { write!(f, "CloseTcp, fd: {}", token.fd) }
        }
    }
}