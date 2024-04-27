use std::{io, mem};
use std::intrinsics::unlikely;
use std::io::Error;
use std::os::fd::{BorrowedFd, RawFd};
use libc::{CLONE_FILES, SYS_unshare, syscall};
use nix::errno::Errno;
use nix::sys::epoll::{Epoll, EpollCreateFlags, EpollEvent, EpollFlags, EpollTimeout};
use nix::sys::socket::{accept4, recvfrom, SockFlag};
use nix::unistd::write;
use slab::Slab;
use crate::engine::io::selector::Selector;
use crate::engine::io::sys::unix::epoll::net::setup_connection;
use crate::engine::io::sys::unix::check_error::check_error;
use crate::engine::io::sys::unix::net;
use crate::engine::io::Token;
use crate::engine::local::Scheduler;
use crate::engine::net::TcpStream;

pub(crate) const REQ_BUF_LEN: usize = 64 * 1024;
pub(crate) const RES_BUF_LEN: usize = 64 * 1024;
pub(crate) const MAX_EPOLL_EVENTS_RETURNED: usize = 256;

// We need to keep the size of the struct as small as possible to cache it.
pub(crate) struct EpolledSelector {
    epoll: Epoll,
    tokens: Slab<Token>,
    unhandled_tokens: Vec<usize>,
    events: [EpollEvent; MAX_EPOLL_EVENTS_RETURNED],
    req_buf: [u8; REQ_BUF_LEN],
    res_buf: [u8; RES_BUF_LEN]
}

impl EpolledSelector {
    pub(crate) fn new() -> io::Result<Self> {
        let epoll = Epoll::new(EpollCreateFlags::empty())?;
        unsafe { check_error(syscall(SYS_unshare, CLONE_FILES), "failed to set CLONE_FILES", true) };

        Ok(EpolledSelector {
            epoll,
            tokens: Slab::with_capacity(64),
            unhandled_tokens: Vec::with_capacity(8),
            events: [EpollEvent::empty(); MAX_EPOLL_EVENTS_RETURNED],
            req_buf: [0;  REQ_BUF_LEN],
            res_buf: [0; RES_BUF_LEN]
        })
    }

    #[inline(always)]
    fn handle_token(&mut self, token: Token, scheduler: &mut Scheduler) {
        match token {
            Token::Empty(_) => {}

            Token::AcceptTcp(token) => {
                let res = accept4(token.fd, SockFlag::SOCK_CLOEXEC);
                if res.is_err() {
                    let err = res.unwrap_err();
                    if err == Errno::EAGAIN || err == Errno::EWOULDBLOCK {
                        return;
                    }
                    unsafe {
                        *token.result = Err(Error::from(err));
                        scheduler.handle_coroutine_state(self, token.coroutine);
                        return;
                    }
                }

                let incoming_fd = res.unwrap();
                unsafe { setup_connection(&BorrowedFd::borrow_raw(incoming_fd)); }
                let token_id = self.tokens.insert(Token::new_empty(incoming_fd));
                unsafe {
                    *token.result = Ok(TcpStream::new(token_id));
                }

                scheduler.handle_coroutine_state(self, token.coroutine);
            }

            Token::PollTcp(token) => {
                let res = recvfrom::<()>(token.fd, &mut self.req_buf);
                if res.is_err() {
                    unsafe {
                        *token.result = Err(Error::from(res.unwrap_err_unchecked()));
                        scheduler.handle_coroutine_state(self, token.coroutine);
                        return;;
                    }
                }

                let (n, _) = res.unwrap();
                unsafe {
                    *token.result = Ok(mem::transmute(&self.req_buf[..n]));
                }

                scheduler.handle_coroutine_state(self, token.coroutine);
            }

            Token::ReadTcp(_) => {
                panic!("[BUG] Epolled Selector handled Token::ReadTcp. Please report this issue.");
            }

            Token::WriteTcp(token) => {
                let fd = token.fd;
                let res = unsafe { write(BorrowedFd::borrow_raw(fd), token.buffer.as_slice()) };

                unsafe {
                    if res.is_ok() {
                        *token.result = Ok(res.unwrap_unchecked());
                    } else {
                        *token.result = Err(Error::from( unsafe { res.unwrap_err_unchecked() }));
                    }
                }

                scheduler.handle_coroutine_state(self, token.coroutine);
            }

            Token::WriteAllTcp(token) => {
                let fd = token.fd;
                let slice = token.buffer.as_slice();
                let mut wrote = 0;
                let mut res;
                loop {
                    res = unsafe { write(BorrowedFd::borrow_raw(fd), &slice[wrote..]) };
                    if unlikely(res.is_err()) {
                        unsafe {
                            *token.result = Err(Error::from(res.unwrap_err_unchecked()));
                            scheduler.handle_coroutine_state(self, token.coroutine);
                            return;
                        };
                    }

                    wrote += unsafe { res.unwrap_unchecked() };
                    if wrote == slice.len() {
                        break;
                    }
                }
                unsafe {
                    *token.result = Ok(());
                }

                scheduler.handle_coroutine_state(self, token.coroutine)
            }
        }
    }
}

impl Selector for EpolledSelector {
    #[inline(always)]
    fn insert_token(&mut self, token: Token) -> usize {
        self.tokens.insert(token)
    }

    #[inline(always)]
    fn get_token_mut_ref(&mut self, token_id: usize) -> &mut Token {
        unsafe { self.tokens.get_unchecked_mut(token_id) }
    }

    #[inline(always)]
    fn poll(&mut self, scheduler: &mut Scheduler) -> Result<(), ()> {
        // TODO maybe drain is faster?
        for i in 0..self.unhandled_tokens.len() {
            let token_id = self.unhandled_tokens[i];
            let old_token = unsafe { self.tokens.get_mut(token_id).unwrap_unchecked() };
            let mut token = Token::new_empty(old_token.fd());
            mem::swap(old_token, &mut token);
            self.handle_token(token, scheduler);
        }
        self.unhandled_tokens.clear();

        let num_incoming_events = self.epoll.wait(&mut self.events, EpollTimeout::try_from(1).unwrap()).expect("failed to wait");
        if num_incoming_events == 0 {
            return Ok(());
        }

        for i in 0..num_incoming_events {
            let event = &self.events[i];
            let Some(token_id) = self.tokens.get_mut(event.data() as usize) else {
                continue;
            };

            let mut token = Token::new_empty(token_id.fd());
            mem::swap(token_id, &mut token);
            self.handle_token(token, scheduler);
        }
        Ok(())
    }

    #[inline(always)]
    fn register(&mut self, fd: RawFd, token_id: usize) {
        unsafe {
            self.epoll.add(BorrowedFd::borrow_raw(fd), EpollEvent::new(EpollFlags::EPOLLIN, token_id as u64)).expect("failed to add fd to epoll");
        }
    }

    #[inline(always)]
    fn deregister(&mut self, token_id: usize) -> Token {
        let token = self.tokens.remove(token_id);
        unsafe {
            self.epoll.delete(BorrowedFd::borrow_raw(token.fd())).expect("failed to remove fd from epoll");
        }
        token
    }

    fn write(&mut self, token_id: usize) {
        self.unhandled_tokens.push(token_id);
    }

    fn write_all(&mut self, token_id: usize) {
        self.unhandled_tokens.push(token_id);
    }

    #[inline(always)]
    fn close_connection(token: Token) {
        unsafe { net::close_connection(&BorrowedFd::borrow_raw(token.fd())) };
    }
}