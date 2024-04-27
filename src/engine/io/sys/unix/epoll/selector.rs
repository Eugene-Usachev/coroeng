use std::{io, mem};
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
            events: [EpollEvent::empty(); MAX_EPOLL_EVENTS_RETURNED],
            req_buf: [0;  REQ_BUF_LEN],
            res_buf: [0; RES_BUF_LEN]
        })
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
        let num_incoming_events = self.epoll.wait(&mut self.events, EpollTimeout::try_from(1).unwrap()).expect("failed to wait");
        if num_incoming_events == 0 {
            return Ok(());
        }

        for i in 0..num_incoming_events {
            let event = &self.events[i];
            let Some(token_ref) = self.tokens.get_mut(event.data() as usize) else {
                continue;
            };

            let mut token = Token::Empty(token_ref.fd());
            mem::swap(token_ref, &mut token);
            match token {
                Token::Empty(_) => {}

                Token::AcceptTcp(fd, coroutine, result) => {
                    let res = accept4(fd, SockFlag::SOCK_CLOEXEC);
                    if res.is_err() {
                        let err = res.unwrap_err();
                        if err == Errno::EAGAIN || err == Errno::EWOULDBLOCK {
                            continue;
                        }
                        unsafe {
                            *result = Err(Error::from(err));
                            scheduler.handle_coroutine_state(self, coroutine);
                            continue;
                        }
                    }

                    let incoming_fd = res.unwrap();
                    unsafe { setup_connection(&BorrowedFd::borrow_raw(incoming_fd)); }
                    let token_id = self.tokens.insert(Token::new_empty(incoming_fd));
                    unsafe {
                        *result = Ok(TcpStream::new(token_id));
                    }

                    scheduler.handle_coroutine_state(self, coroutine);
                }

                Token::PollTcp(fd, coroutine, result) => {
                    let res = recvfrom::<()>(fd, &mut self.req_buf);
                    if res.is_err() {
                        unsafe {
                            *result = Err(Error::from(res.unwrap_err_unchecked()));
                            scheduler.handle_coroutine_state(self, coroutine);
                            continue;
                        }
                    }

                    let (n, _) = res.unwrap();
                    unsafe {
                        *result = Ok(mem::transmute(&self.req_buf[..n]));
                    }

                    scheduler.handle_coroutine_state(self, coroutine);
                }

                Token::ReadTcp(_, _, _, _) => {
                    panic!("[BUG] Epolled Selector handled Token::ReadTcp. Please report this issue.");
                }

                Token::WriteTcp(_, _, _, _) => {
                    panic!("[BUG] Epolled Selector handled Token::WriteTcp. Please report this issue.");
                }
            }
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

    fn write(&mut self, token_id: usize, buf: &[u8]) -> Result<usize, io::Error> {
        let token = unsafe { self.tokens.get_unchecked_mut(token_id) };
        let fd = token.fd();
        let res = unsafe { write(BorrowedFd::borrow_raw(fd), buf) };
        if res.is_ok() {
            return Ok(unsafe { res.unwrap_unchecked() });
        }

        unsafe { Err(std::io::Error::from(res.unwrap_err_unchecked())) }
    }

    #[inline(always)]
    fn close_connection(token: Token) {
        unsafe { net::close_connection(&BorrowedFd::borrow_raw(token.fd())) };
    }
}