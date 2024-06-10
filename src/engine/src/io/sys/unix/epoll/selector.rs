use std::{io, mem};
use std::intrinsics::unlikely;
use std::io::Error;
use std::os::fd::{BorrowedFd, RawFd};
use libc::{CLONE_FILES, SYS_unshare, syscall};
use nix::errno::Errno;
use nix::sys::epoll::{Epoll, EpollCreateFlags, EpollEvent, EpollFlags, EpollTimeout};
use nix::sys::socket::{accept4, recvfrom, SockFlag};
use nix::unistd::write;
use crate::io::selector::Selector;
use crate::io::sys::unix::epoll::net::setup_connection;
use crate::io::sys::unix::check_error::check_error;
use crate::io::sys::unix::net;
use crate::io::State;
use crate::scheduler::Scheduler;
use crate::net::TcpStream;
use crate::{write_err, write_ok};
use crate::utils::Ptr;

pub(crate) const REQ_BUF_LEN: usize = 64 * 1024;
pub(crate) const MAX_EPOLL_EVENTS_RETURNED: usize = 256;

// We need to keep the size of the struct as small as possible to cache it.
pub(crate) struct EpolledSelector {
    epoll: Epoll,
    unhandled_states: Vec<Ptr<State>>,
    events: [EpollEvent; MAX_EPOLL_EVENTS_RETURNED],
    req_buf: [u8; REQ_BUF_LEN]
}

impl EpolledSelector {
    pub(crate) fn new() -> io::Result<Self> {
        let epoll = Epoll::new(EpollCreateFlags::empty())?;
        unsafe { check_error(syscall(SYS_unshare, CLONE_FILES), "failed to set CLONE_FILES", true) };

        Ok(EpolledSelector {
            epoll,
            unhandled_states: Vec::with_capacity(8),
            events: [EpollEvent::empty(); MAX_EPOLL_EVENTS_RETURNED],
            req_buf: [0;  REQ_BUF_LEN]
        })
    }

    #[inline(always)]
    #[must_use]
    fn handle_state(&mut self, state_ptr: Ptr<State>, scheduler: &mut Scheduler) -> bool {
        let state = unsafe { state_ptr.read() };
        match state {
            State::Empty(_) => { false }

            State::AcceptTcp(state) => {
                let res = accept4(state.fd, SockFlag::SOCK_CLOEXEC);
                if res.is_err() {
                    let err = res.unwrap_err();
                    if err == Errno::EAGAIN || err == Errno::EWOULDBLOCK {
                        return false;
                    }
                    write_err!(state.result, Error::from(err));
                    return scheduler.handle_coroutine_state(self, state.coroutine);
                }

                let incoming_fd = res.unwrap();
                unsafe { setup_connection(&BorrowedFd::borrow_raw(incoming_fd)); }
                write_ok!(state.result, TcpStream::new(incoming_fd));

                scheduler.handle_coroutine_state(self, state.coroutine)
            }

            State::ConnectTcp(_state) => {
                todo!();
            }

            State::PollTcp(state) => {
                let res = recvfrom::<()>(state.fd, &mut self.req_buf);
                if res.is_err() {
                    write_err!(state.result, Error::from(res.unwrap_err_unchecked()));
                    return scheduler.handle_coroutine_state(self, state.coroutine)
                }

                let (n, _) = res.unwrap();
                write_ok!(state.result, mem::transmute(&self.req_buf[..n]));

                scheduler.handle_coroutine_state(self, state.coroutine)
            }

            State::ReadTcp(_) => {
                panic!("[BUG] Epolled Selector handled State::ReadTcp. Please report this issue.");
            }

            State::WriteTcp(mut state) => {
                let fd = state.fd;
                let res = unsafe { write(BorrowedFd::borrow_raw(fd), state.buffer.as_ref()) };

                if res.is_ok() {
                    let written = unsafe { res.unwrap_unchecked() };
                    if written == state.buffer.len() {
                        write_ok!(state.result, None);
                    } else {
                        state.buffer.set_offset(state.buffer.offset() + written);
                        write_ok!(state.result, Some(state.buffer));
                    }
                } else {
                    write_err!(state.result, Error::from(res.unwrap_err_unchecked()));
                }

                scheduler.handle_coroutine_state(self, state.coroutine)
            }

            State::WriteAllTcp(mut state) => {
                let fd = state.fd;
                let mut res;
                loop {
                    res = unsafe { write(BorrowedFd::borrow_raw(fd), state.buffer.as_ref()) };
                    if unlikely(res.is_err()) {
                        write_err!(state.result, Error::from(res.unwrap_err_unchecked()));
                        scheduler.handle_coroutine_state(self, state.coroutine);
                        return false;
                    }

                    state.buffer.set_offset(state.buffer.offset() + unsafe { res.unwrap_unchecked() });
                    if state.buffer.len() == 0 {
                        break;
                    }
                }
                write_ok!(state.result, ());

                scheduler.handle_coroutine_state(self, state.coroutine)
            }

            State::CloseTcp(state) => {
                let fd = state.fd;
                let _ = self.deregister(state.fd);
                unsafe { net::close_connection(&BorrowedFd::borrow_raw(fd)); }
                scheduler.handle_coroutine_state(self, state.coroutine)
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
    fn poll(&mut self, scheduler: &mut Scheduler) -> Result<bool, ()> {
        // TODO maybe drain is faster?
        for i in 0..self.unhandled_states.len() {
            let state_ptr = self.unhandled_states[i];
            if unlikely(self.handle_state(state_ptr, scheduler)) {
                return Ok(true);
            }
        }
        self.unhandled_states.clear();

        let num_incoming_events = self.epoll.wait(&mut self.events, EpollTimeout::try_from(1).unwrap()).expect("failed to wait");
        if num_incoming_events == 0 {
            return Ok(false);
        }

        for i in 0..num_incoming_events {
            let event = &self.events[i];
            if unlikely(self.handle_state(Ptr::from(event.data()), scheduler)) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    #[inline(always)]
    fn register(&mut self, state_ptr: Ptr<State>) {
        let fd = unsafe { state_ptr.as_ref() }.fd();
        let res = unsafe {
            self.epoll.add(BorrowedFd::borrow_raw(fd), EpollEvent::new(EpollFlags::EPOLLIN, state_ptr.as_u64()))
        };

        if res.is_err() {
            panic!("failed to add fd to epoll: {} for fd: {}", res.unwrap_err(), fd);
        }
    }

    #[inline(always)]
    fn deregister(&mut self, fd: RawFd) {
        unsafe {
            self.epoll.delete(BorrowedFd::borrow_raw(fd)).expect("failed to remove fd from epoll");
        }
    }

    fn write(&mut self, state_ref: Ptr<State>) {
        self.unhandled_states.push(state_ref);
    }

    fn write_all(&mut self, state_ref: Ptr<State>) {
        self.unhandled_states.push(state_ref);
    }

    #[inline(always)]
    fn close_connection(&mut self, state_ref: Ptr<State>) {
        self.unhandled_states.push(state_ref);
    }
}