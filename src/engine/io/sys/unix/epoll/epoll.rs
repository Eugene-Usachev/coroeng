use std::{io, mem};
use std::ops::Coroutine;
use std::os::fd::{BorrowedFd, RawFd};
use nix::errno::Errno;
use nix::sys::epoll::{Epoll, EpollCreateFlags, EpollEvent, EpollFlags, EpollTimeout};
use nix::sys::socket::{accept4, recvfrom, SockFlag};
use slab::Slab;
use crate::engine::io::sys::unix::epoll::net;
use crate::engine::net::tcp::TcpStream;
use crate::engine::io::sys::unix::epoll::net::setup_connection;
use crate::engine::io::token::Token;
use crate::engine::work_stealing::Scheduler;

pub struct Selector<'scheduler> {
    pub(crate) epoll: Epoll,
    scheduler: &'scheduler Scheduler,
    pub(crate) tokens: Slab<Token>,
    events: [EpollEvent; MAX_EPOLL_EVENTS_RETURNED],
    req_buf: [u8; REQ_BUF_LEN],
    res_buf: [u8; RES_BUF_LEN]
}

impl<'scheduler> Selector<'scheduler> {
    pub(crate) fn new(scheduler: &'scheduler Scheduler) -> io::Result<Self> {
        let epoll = Epoll::new(EpollCreateFlags::empty())?;

        Ok(Selector {
            epoll,
            scheduler,
            tokens: Slab::with_capacity(64),
            events: [EpollEvent::empty(); MAX_EPOLL_EVENTS_RETURNED],
            req_buf: [0; REQ_BUF_LEN],
            res_buf: [0; RES_BUF_LEN]
        })
    }

    #[inline(always)]
    pub fn poll(&mut self) -> Result<(), ()> {
        let num_incoming_events = self.epoll.wait(&mut self.events, EpollTimeout::try_from(-1).unwrap()).expect("failed to wait");
        if num_incoming_events == 0 {
            return Ok(());
        }

        for i in 0..num_incoming_events {
            let event = &self.events[i];
            let Some(token) = self.tokens.get_mut(event.data() as usize) else {
                continue;
            };
            let mut new_token = Token::new_empty(token.fd());
            mem::swap(token, &mut new_token);

            match new_token {
                Token::Empty(_) => {}

                Token::AcceptTcp(fd, mut coroutine, result) => {
                    let res = accept4(fd, SockFlag::SOCK_CLOEXEC);
                    if res.is_err() {
                        let err = res.unwrap_err();
                        if err == Errno::EAGAIN || err == Errno::EWOULDBLOCK {
                            continue;
                        }
                        unsafe {
                            *result = Err("accept failed");
                            Scheduler::handle_coroutine_state(self.scheduler, self, coroutine.as_mut().resume(()), coroutine);
                            continue;
                        }
                    }

                    let incoming_fd = res.unwrap();
                    unsafe { setup_connection(&BorrowedFd::borrow_raw(incoming_fd)); }
                    let token_id = self.tokens.insert(Token::new_empty(incoming_fd));
                    // register
                    unsafe {
                        *result = Ok(TcpStream {
                            token_id,
                            is_registered: false
                        });
                    }
                    Scheduler::handle_coroutine_state(self.scheduler, self, coroutine.as_mut().resume(()), coroutine);
                }

                Token::ReadTcp(fd, mut coroutine, result) => {
                    let res = recvfrom::<()>(fd, &mut self.req_buf);
                    if res.is_err() {
                        unsafe {
                            *result = Err("read failed");
                            Scheduler::handle_coroutine_state(self.scheduler, self, coroutine.as_mut().resume(()), coroutine);
                            continue;
                        }
                    }

                    let (n, _) = res.unwrap();
                    unsafe {
                        *result = Ok(mem::transmute(&self.req_buf[..n]));
                    }

                    Scheduler::handle_coroutine_state(self.scheduler, self, coroutine.as_mut().resume(()), coroutine);
                }
            }
        }
        Ok(())
    }

    #[inline(always)]
    pub fn register(&mut self, fd: RawFd, token_id: usize) {
        unsafe {
            self.epoll.add(BorrowedFd::borrow_raw(fd), EpollEvent::new(EpollFlags::EPOLLIN, token_id as u64)).expect("failed to add fd to epoll");
        }
    }

    #[inline(always)]
    pub fn deregister(&mut self, token_id: usize) {
        let token = self.tokens.remove(token_id);
        unsafe {
            self.epoll.delete(BorrowedFd::borrow_raw(token.fd())).expect("failed to remove fd from epoll");
        }
    }

    #[inline(always)]
    pub fn deregister_token(&mut self, token: Token) {
        unsafe {
            self.epoll.delete(BorrowedFd::borrow_raw(token.fd())).expect("failed to remove fd from epoll");
        }
    }
}