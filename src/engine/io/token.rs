use std::io::Error;
use std::fmt::{Debug, Formatter};
use std::os::fd::{RawFd};
use crate::engine::coroutine::coroutine::CoroutineImpl;
use crate::engine::net::tcp::TcpStream;
use crate::utils::Buffer;

pub(crate) enum Token {
    Empty(RawFd),
    AcceptTcp(RawFd, CoroutineImpl, *mut Result<TcpStream, Error>),
    PollTcp(RawFd, CoroutineImpl, *mut Result<&'static [u8], Error>),
    ReadTcp(RawFd, Buffer, CoroutineImpl, *mut Result<&'static [u8], Error>),
    WriteTcp(RawFd, Buffer, CoroutineImpl, *mut Result<usize, Error>)
}

impl Token {
    pub(crate) fn fd(&self) -> RawFd {
        match self {
            Token::Empty(fd) => { *fd }
            Token::AcceptTcp(fd, _, _) => { *fd }
            Token::PollTcp(fd, _, _) => { *fd }
            Token::ReadTcp(fd, _, _, _) => { *fd }
            Token::WriteTcp(fd, _, _, _) => { *fd }
        }
    }

    pub(crate) fn new_empty(fd: RawFd) -> Self {
        Token::Empty(fd)
    }

    pub(crate) fn new_accept_tcp(listener: RawFd, coroutine: CoroutineImpl, result: *mut Result<TcpStream, Error>) -> Self {
        Token::AcceptTcp(listener, coroutine, result)
    }

    pub(crate) fn new_poll_tcp(stream: RawFd, coroutine: CoroutineImpl, result: *mut Result<&'_ [u8], Error>) -> Self {
        let result = unsafe { std::mem::transmute(result) };
        Token::PollTcp(stream, coroutine, result)
    }

    pub(crate) fn new_read_tcp(stream: RawFd, buf: Buffer, coroutine: CoroutineImpl, result: *mut Result<&'_ [u8], Error>) -> Self {
        let result = unsafe { std::mem::transmute(result) };
        Token::ReadTcp(stream, buf, coroutine, result)
    }

    pub(crate) fn new_write_tcp(stream: RawFd, buf: Buffer, coroutine: CoroutineImpl, result: *mut Result<usize, Error>) -> Self {
        Token::WriteTcp(stream, buf, coroutine, result)
    }
}

impl Debug for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Empty(fd) => { write!(f, "Empty, fd: {}", fd) }
            Token::AcceptTcp(fd, _, _) => { write!(f, "AcceptTcp, fd: {}", fd) }
            Token::PollTcp(fd, _, _) => { write!(f, "ReadTcp, fd: {}", fd) }
            Token::ReadTcp(fd, _, _, _) => { write!(f, "ReadTcp, fd: {}", fd) }
            Token::WriteTcp(fd, _, _, _) => { write!(f, "WriteTcp, fd: {}", fd) }
        }
    }
}