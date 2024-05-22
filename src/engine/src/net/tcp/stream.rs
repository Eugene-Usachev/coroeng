use std::io::Error;
use std::os::fd::RawFd;
use proc::coro;
use crate::coroutine::{CoroutineImpl, YieldStatus};
use crate::io::State;
use crate::{local_scheduler, spawn_local};
use crate::utils::{Buffer, Ptr};

pub struct TcpStream {
    is_registered: bool,
    data: Ptr<State>
}

impl TcpStream {
    pub fn new(fd: RawFd) -> Self {
        Self {
            is_registered: false,
            data: Ptr::new(State::new_empty(fd))
        }
    }

    #[inline(always)]
    pub fn state_ptr(&mut self) -> Ptr<State> {
        self.data
    }

    pub fn is_registered(&self) -> bool {
        self.is_registered
    }

    pub fn set_registered(&mut self, is_registered: bool) {
        self.is_registered = is_registered;
    }

    pub fn read(&mut self, res: *mut Result<&'static [u8], Error>) -> YieldStatus {
        let is_registered = self.is_registered();
        if !is_registered {
            self.set_registered(true);
        }
        YieldStatus::tcp_read(is_registered, self.data, res)
    }

    pub fn write(&mut self, buf: Buffer, res: *mut Result<usize, Error>) -> YieldStatus {
        YieldStatus::tcp_write(self.data, buf, res)
    }

    pub fn write_all(&mut self, buf: Buffer, res: *mut Result<(), Error>) -> YieldStatus {
        YieldStatus::tcp_write_all(self.data, buf, res)
    }

    fn close(state_ref: Ptr<State>) -> YieldStatus {
        YieldStatus::tcp_close(state_ref)
    }
}

fn close_stream(state_ref: Ptr<State>) -> CoroutineImpl {
    Box::pin(#[coroutine] static move || {
        yield TcpStream::close(state_ref);
        unsafe { state_ref.drop_in_place(); }
    })
}

impl Drop for TcpStream {
    fn drop(&mut self) {
        let state_ptr = self.data;
        if self.is_registered() {
            local_scheduler().sched(close_stream(state_ptr));
        } else {
            unsafe { state_ptr.drop_in_place(); }
        }
    }
}