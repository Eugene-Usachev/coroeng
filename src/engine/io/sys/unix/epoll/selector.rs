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
use crate::{write_err, write_ok};

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
    fn handle_token(&mut self, token_id: usize, scheduler: &mut Scheduler) {
        let token_ref = unsafe { self.tokens.get_unchecked_mut(token_id) };
        let mut token = Token::new_empty(token_ref.fd());
        unsafe { mem::swap(token_ref, &mut token); }
        match token {
            Token::Empty(_) => {}

            Token::AcceptTcp(token) => {
                let res = accept4(token.fd, SockFlag::SOCK_CLOEXEC);
                if res.is_err() {
                    let err = res.unwrap_err();
                    if err == Errno::EAGAIN || err == Errno::EWOULDBLOCK {
                        return;
                    }
                    write_err!(token.result, Error::from(err));
                    scheduler.handle_coroutine_state(self, token.coroutine);
                    return;
                }

                let incoming_fd = res.unwrap();
                unsafe { setup_connection(&BorrowedFd::borrow_raw(incoming_fd)); }
                let token_id = self.tokens.insert(Token::new_empty(incoming_fd));
                write_ok!(token.result, TcpStream::new(token_id));

                scheduler.handle_coroutine_state(self, token.coroutine);
            }

            Token::PollTcp(token) => {
                let res = recvfrom::<()>(token.fd, &mut self.req_buf);
                if res.is_err() {
                    write_err!(token.result, Error::from(res.unwrap_err_unchecked()));
                    scheduler.handle_coroutine_state(self, token.coroutine);
                    return;
                }

                let (n, _) = res.unwrap();
                write_ok!(token.result, mem::transmute(&self.req_buf[..n]));

                scheduler.handle_coroutine_state(self, token.coroutine);
            }

            Token::ReadTcp(_) => {
                panic!("[BUG] Epolled Selector handled Token::ReadTcp. Please report this issue.");
            }

            Token::WriteTcp(token) => {
                let fd = token.fd;
                let res = unsafe { write(BorrowedFd::borrow_raw(fd), token.buffer.as_slice()) };

                if res.is_ok() {
                    write_ok!(token.result, res.unwrap_unchecked());
                } else {
                    write_err!(token.result, Error::from(res.unwrap_err_unchecked()));
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
                        write_err!(token.result, Error::from(res.unwrap_err_unchecked()));
                        scheduler.handle_coroutine_state(self, token.coroutine);
                        return;
                    }

                    wrote += unsafe { res.unwrap_unchecked() };
                    if wrote == slice.len() {
                        break;
                    }
                }
                write_ok!(token.result, ());

                scheduler.handle_coroutine_state(self, token.coroutine)
            }

            Token::CloseTcp(token) => {
                let fd = token.fd;
                let _ = self.deregister(token_id);
                unsafe { net::close_connection(&BorrowedFd::borrow_raw(fd)); }
                scheduler.handle_coroutine_state(self, token.coroutine);
            }
        }
    }
}

impl Selector for EpolledSelector {
    #[inline(always)]
    fn need_reregister(&self) -> bool {
        false
    }

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
            self.handle_token(token_id, scheduler);
        }
        self.unhandled_tokens.clear();

        let num_incoming_events = self.epoll.wait(&mut self.events, EpollTimeout::try_from(1).unwrap()).expect("failed to wait");
        if num_incoming_events == 0 {
            return Ok(());
        }

        for i in 0..num_incoming_events {
            let event = &self.events[i];
            self.handle_token(event.data() as usize, scheduler);
        }
        Ok(())
    }

    #[inline(always)]
    fn register(&mut self, token_id: usize) {
        let fd = self.tokens[token_id].fd();
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
    fn close_connection(&mut self, token_id: usize) {
        self.unhandled_tokens.push(token_id);
    }
}