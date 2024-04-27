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
    fd: RawFd,
    buffer: Buffer,
    coroutine: CoroutineImpl,
    result: *mut Result<&'static [u8], Error>
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
    pub(crate) result: *mut Result<(), Error>
}

pub(crate) enum Token {
    Empty(EmptyToken),
    AcceptTcp(AcceptTcpToken),
    PollTcp(PollTcpToken),
    ReadTcp(ReadTcpToken),
    WriteTcp(WriteTcpToken),
    WriteAllTcp(WriteAllTcpToken)
}

impl Token {
    pub(crate) fn fd(&self) -> RawFd {
        match self {
            Token::Empty(token) => { token.fd }
            Token::AcceptTcp(token) => { token.fd }
            Token::PollTcp(token) => { token.fd }
            Token::ReadTcp(token) => { token.fd }
            Token::WriteTcp(token) => { token.fd }
            Token::WriteAllTcp(token) => { token.fd }
        }
    }

    pub(crate) fn new_empty(fd: RawFd) -> Self {
        Token::Empty(EmptyToken { fd })
    }

    pub(crate) fn new_accept_tcp(listener: RawFd, coroutine: CoroutineImpl, result: *mut Result<TcpStream, Error>) -> Self {
        Token::AcceptTcp(AcceptTcpToken { fd: listener, coroutine, result })
    }

    pub(crate) fn new_poll_tcp(stream: RawFd, coroutine: CoroutineImpl, result: *mut Result<&'_ [u8], Error>) -> Self {
        let result = unsafe { std::mem::transmute(result) };
        Token::PollTcp(PollTcpToken { fd: stream, coroutine, result })
    }

    pub(crate) fn new_read_tcp(stream: RawFd, buf: Buffer, coroutine: CoroutineImpl, result: *mut Result<&'_ [u8], Error>) -> Self {
        let result = unsafe { std::mem::transmute(result) };
        Token::ReadTcp(ReadTcpToken { fd: stream, buffer: buf, coroutine, result })
    }

    pub(crate) fn new_write_tcp(stream: RawFd, buf: Buffer, coroutine: CoroutineImpl, result: *mut Result<usize, Error>) -> Self {
        Token::WriteTcp(WriteTcpToken { fd: stream, buffer: buf, coroutine, result })
    }

    pub(crate) fn new_write_all_tcp(stream: RawFd, buf: Buffer, coroutine: CoroutineImpl, result: *mut Result<(), Error>) -> Self {
        Token::WriteAllTcp(WriteAllTcpToken { fd: stream, buffer: buf, coroutine, result })
    }
}

impl Debug for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Empty(token) => { write!(f, "Empty, fd: {}", token.fd) }
            Token::AcceptTcp(token) => { write!(f, "AcceptTcp, fd: {}", token.fd) }
            Token::PollTcp(token) => { write!(f, "ReadTcp, fd: {}", token.fd) }
            Token::ReadTcp(token) => { write!(f, "ReadTcp, fd: {}", token.fd) }
            Token::WriteTcp(token) => { write!(f, "WriteTcp, fd: {}", token.fd) }
            Token::WriteAllTcp(token) => { write!(f, "WriteAllTcp, fd: {}", token.fd) }
        }
    }
}