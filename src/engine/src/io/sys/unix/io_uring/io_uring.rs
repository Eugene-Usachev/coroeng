use std::collections::VecDeque;
use std::cell::UnsafeCell;
use std::io::Error;
use std::os::fd::{AsRawFd, IntoRawFd, RawFd};
use std::{mem, ptr};
use io_uring::{cqueue, IoUring, opcode, squeue, types};
use io_uring::types::{SubmitArgs, Timespec};
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use crate::buf::buffer;
use crate::io::{Selector, PollState};
use crate::net::TcpStream;
use crate::scheduler::Scheduler;
use crate::utils::{Ptr};
use crate::{write_err, write_ok};

macro_rules! handle_ret {
    ($ret: expr, $state: expr, $scheduler: expr, $selector: expr) => {
        if $ret < 0 {
            let err = Error::last_os_error();
            unsafe { $state.result.write(Err(err)); }
            $scheduler.handle_coroutine_state($selector, $state.coroutine);
            return;
        }
    };
}

macro_rules! handle_ret_without_result {
    ($ret: expr, $state: expr, $scheduler: expr, $selector: expr) => {
        if $ret < 0 {
            $scheduler.handle_coroutine_state($selector, $state.coroutine);
            return;
        }
    };
}

const TIMEOUT: Timespec = Timespec::new().nsec(500_000);

pub(crate) struct IoUringSelector {
    timeout: SubmitArgs<'static, 'static>,
    /// # Why we need some cell?
    ///
    /// We can't rewrite engine ([`Selector`] trait) to use separately `ring` field and other fields in different methods.
    /// For example, we can't use `&mut self` in [`Scheduler::handle_coroutine_state`] function, because we are borrowing the `ring` before [`Scheduler::handle_coroutine_state`].
    /// So we need some cell not to destroy the abstraction.
    ///
    /// # Why we use UnsafeCell?
    ///
    /// Because we can guarantee that:
    /// * only one thread can borrow the [`IoUringSelector`] at the same time
    /// * only in the [`poll`] method we borrow the `ring` field for [`CompletionQueue`] and [`SubmissionQueue`],
    /// but only after the [`SubmissionQueue`] is submitted we start using the [`CompletionQueue`] that can call the [`IoUringSelector::push_sqe`]
    /// but it is safe, because the [`SubmissionQueue`] has already been read and submitted.
    ring: UnsafeCell<IoUring<squeue::Entry, cqueue::Entry>>,
    backlog: VecDeque<squeue::Entry>
}

impl IoUringSelector {
    pub fn new() -> Self {
        println!("io_uring");
        Self {
            timeout: SubmitArgs::new().timespec(&TIMEOUT),
            ring: UnsafeCell::new(IoUring::new(1024).unwrap()),
            backlog: VecDeque::with_capacity(64)
        }
    }

    #[inline(always)]
    fn add_sqe(&mut self, sqe: squeue::Entry) {
        let ring = unsafe { &mut *self.ring.get() };
        unsafe {
            if ring.submission().push(&sqe).is_err() {
                self.backlog.push_back(sqe);
            }
        }
    }

    #[inline(always)]
    fn submit(&mut self) -> Result<(), Error> {
        let ring = unsafe { &mut *self.ring.get() };
        let mut sq = unsafe { ring.submission_shared() };
        let submitter = ring.submitter();

        loop {
            if sq.is_full() {
                match submitter.submit() {
                    Ok(_) => (),
                    Err(ref err) if err.raw_os_error() == Some(libc::EBUSY) => break,
                    Err(err) => return Err(err.into()),
                }
            }
            sq.sync();

            match self.backlog.pop_front() {
                Some(sqe) => unsafe {
                    let _ = sq.push(&sqe);
                },
                None => break,
            }
        }

        match submitter.submit_with_args(1, &self.timeout) {
            Ok(_) => (),
            Err(ref err) if err.raw_os_error() == Some(libc::ETIME) => (),
            Err(ref err) if err.raw_os_error() == Some(libc::EBUSY) => (),
            Err(err) => return Err(err.into()),
        };

        Ok(())
    }

    #[inline(always)]
    fn handle_completion(&mut self, scheduler: &mut Scheduler, ret: i32, ptr: Ptr<PollState>) {
        let state = unsafe { ptr.read() };

        match state {
            PollState::Empty(_) => {
                panic!("[BUG] tried to handle an empty state in [`IoUringSelector`]. Please report this issue.")
            }
            PollState::AcceptTcp(state) => {
                handle_ret!(ret, state, scheduler, self);

                let accepted_fd = ret;
                write_ok!(state.result, TcpStream::new(accepted_fd));

                scheduler.handle_coroutine_state(self, state.coroutine);
            }
            PollState::ConnectTcp(state) => {
                handle_ret!(ret, state, scheduler, self);

                write_ok!(state.result, TcpStream::new(state.socket.into_raw_fd()));

                scheduler.handle_coroutine_state(self, state.coroutine);
                unsafe { ptr.drop_in_place() };
            }
            PollState::PollTcp(state) => {
                handle_ret!(ret, state, scheduler, self);

                unsafe { ptr.write(PollState::new_read_tcp(state.fd, buffer(), state.coroutine, state.result)) };

                self.register(ptr);
            }
            PollState::ReadTcp(state) => {
                handle_ret!(ret, state, scheduler, self);

                let slice = unsafe { mem::transmute(&state.buffer.slice[..ret as usize]) };
                write_ok!(state.result, slice);

                scheduler.handle_coroutine_state(self, state.coroutine);
            }
            PollState::WriteTcp(mut state) => {
                handle_ret!(ret, state, scheduler, self);

                if ret as usize == state.buffer.len() {
                    write_ok!(state.result, None);
                } else {
                    state.buffer.set_offset(state.buffer.offset() + ret as usize);
                    write_ok!(state.result, Some(state.buffer));
                }

                scheduler.handle_coroutine_state(self, state.coroutine);
            }
            PollState::WriteAllTcp(mut state) => {
                handle_ret!(ret, state, scheduler, self);

                if ret as usize == state.buffer.len() {
                    write_ok!(state.result, ());
                    scheduler.handle_coroutine_state(self, state.coroutine);
                } else {
                    state.buffer.set_offset(state.buffer.offset() + ret as usize);
                    unsafe { ptr.write(PollState::new_write_all_tcp(state.fd, state.buffer, state.coroutine, state.result)) };

                    self.register(ptr);
                }
            }
            PollState::CloseTcp(state) => {
                handle_ret_without_result!(ret, state, scheduler, self);

                scheduler.handle_coroutine_state(self, state.coroutine);
            }
        }
    }
}

impl Selector for IoUringSelector {
    #[inline(always)]
    fn need_reregister(&self) -> bool {
        true
    }

    fn deregister(&mut self, _fd: RawFd) {}

    #[inline(always)]
    fn poll(&mut self, scheduler: &mut Scheduler) -> Result<(), ()> {
        if self.submit().is_err() {
            return Err(())
        }

        let ring = unsafe { &mut *self.ring.get() };
        let mut cq = ring.completion();
        cq.sync();

        for cqe in &mut cq {
            let ret = cqe.result();
            let token = Ptr::from(cqe.user_data());
            self.handle_completion(scheduler, ret, token);
        }

        Ok(())
    }

    #[inline(always)]
    fn register(&mut self, state_ptr: Ptr<PollState>) {
        let state = unsafe { state_ptr.as_mut() };

        let mut entry = match state {
            PollState::Empty(_) => { panic!("[BUG] tried to register an empty state in [`IoUringSelector`]. Please report this issue.") }
            PollState::AcceptTcp(state) => {
                opcode::Accept::new(types::Fd(state.fd), ptr::null_mut(), ptr::null_mut())
                    .build()
            }
            PollState::ConnectTcp(state) => {
                opcode::Connect::new(types::Fd(state.socket.as_raw_fd()), state.address.as_ptr(), state.address.len())
                    .build()
            }
            PollState::PollTcp(state) => {
                opcode::PollAdd::new(types::Fd(state.fd), libc::POLLIN as _)
                    .build()
            }
            PollState::ReadTcp(state) => {
                opcode::Recv::new(types::Fd(state.fd), state.buffer.as_mut_ptr(), state.buffer.cap() as _)
                    .build()
            }
            PollState::WriteTcp(state) => {
                opcode::Send::new(types::Fd(state.fd), state.buffer.as_ptr(), state.buffer.len() as _)
                    .build()
            }
            PollState::WriteAllTcp(state) => {
                opcode::Send::new(types::Fd(state.fd), state.buffer.as_ptr(), state.buffer.len() as _)
                    .build()
            }
            PollState::CloseTcp(state) => {
                opcode::Close::new(types::Fixed(state.fd as u32))
                    .build()
            }
        };

        entry = entry.user_data(state_ptr.as_u64());
        self.add_sqe(entry);
    }

    #[inline(always)]
    fn write(&mut self, state_ref: Ptr<PollState>) {
        self.register(state_ref);
    }

    #[inline(always)]
    fn write_all(&mut self, state_ref: Ptr<PollState>) {
        self.register(state_ref);
    }

    #[inline(always)]
    fn close_connection(&mut self, state_ref: Ptr<PollState>) {
        self.register(state_ref);
    }
}