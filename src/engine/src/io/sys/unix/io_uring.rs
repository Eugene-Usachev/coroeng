use std::collections::VecDeque;
use std::cell::UnsafeCell;
use std::io::Error;
use std::os::fd::{AsRawFd, IntoRawFd, RawFd};
use std::{ptr};
use io_uring::{cqueue, IoUring, opcode, squeue, types};
use io_uring::types::{SubmitArgs, Timespec};
use crate::buf::buffer;
use crate::io::{Selector, State};
use crate::net::TcpStream;
use crate::scheduler::Scheduler;
use crate::utils::{Ptr};
use crate::{write_ok};

macro_rules! handle_ret_and_get_state {
    ($ret: expr, $state_ptr: expr, $scheduler: expr, $selector: expr) => {
        {
            let read_state = unsafe { $state_ptr.read() };
            
            if $ret < 0 {
                let err = Error::last_os_error();
                unsafe { read_state.result.write(Err(err)); }
                $scheduler.handle_coroutine_state($selector, read_state.coroutine);
                return;
            }
            
            read_state
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
    fn handle_completion(&mut self, scheduler: &mut Scheduler, ret: i32, mut ptr: Ptr<State>) {
        let state = unsafe { ptr.read() };

        match state {
            State::Empty(_) => {
                panic!("[BUG] tried to handle an empty state in [`IoUringSelector`]. Please report this issue.")
            }
            
            State::AcceptTcp(accept_tcp_state_ptr) => {
                let state = handle_ret_and_get_state!(ret, accept_tcp_state_ptr, scheduler, self);
                let mut new_state_ptr = scheduler.get_state_ptr();
                new_state_ptr.rewrite_state(scheduler.state_manager().empty(ret), scheduler.state_manager());
                write_ok!(state.result, TcpStream::new(new_state_ptr));

                scheduler.handle_coroutine_state(self, state.coroutine);
            }
            
            State::ConnectTcp(connect_tcp_state_ptr) => {
                let state = handle_ret_and_get_state!(ret, connect_tcp_state_ptr, scheduler, self);

                let mut new_state_ptr = scheduler.get_state_ptr();
                new_state_ptr.rewrite_state(scheduler.state_manager().empty(state.socket.into_raw_fd()), scheduler.state_manager());
                
                write_ok!(state.result, TcpStream::new(new_state_ptr));

                scheduler.handle_coroutine_state(self, state.coroutine);
                unsafe { ptr.drop_and_deallocate() };
            }
            
            State::Poll(poll_state_ptr) => {
                let state = handle_ret_and_get_state!(ret, poll_state_ptr, scheduler, self);

                ptr.rewrite_state(scheduler.state_manager().recv(state.fd, buffer(), state.coroutine, state.result), scheduler.state_manager());

                self.register(ptr);
            }
            
            State::Recv(recv_state_ptr) => {
                let mut state = handle_ret_and_get_state!(ret, recv_state_ptr, scheduler, self);

                state.buffer.add_len(ret as usize);
                write_ok!(state.result, state.buffer);

                scheduler.handle_coroutine_state(self, state.coroutine)
            }
            
            State::Send(send_state_ptr) => {
                let mut state = handle_ret_and_get_state!(ret, send_state_ptr, scheduler, self);

                state.buffer.add_offset(ret as usize);
                write_ok!(state.result, state.buffer);

                scheduler.handle_coroutine_state(self, state.coroutine)
            }
            
            State::SendAll(send_all_state_ptr) => {
                let mut state = handle_ret_and_get_state!(ret, send_all_state_ptr, scheduler, self);

                if ret as usize == state.buffer.len() {
                    state.buffer.set_offset_to_len();
                    write_ok!(state.result, state.buffer);
                    scheduler.handle_coroutine_state(self, state.coroutine)
                } else {
                    state.buffer.set_offset(state.buffer.offset() + ret as usize);
                    ptr.rewrite_state(scheduler.state_manager().send_all(state.fd, state.buffer, state.coroutine, state.result), scheduler.state_manager());

                    self.register(ptr);
                }
            }
            
            State::Close(close_state_ptr) => {
                let state = handle_ret_and_get_state!(ret, close_state_ptr, scheduler, self);
                
                write_ok!(state.result, ());
                scheduler.handle_coroutine_state(self, state.coroutine)
            }

            State::Open(open_state_ptr) => {
                todo!();
            }

            State::Read(read_state_ptr) => {
                let mut state = handle_ret_and_get_state!(ret, read_state_ptr, scheduler, self);

                state.buffer.add_len(ret as usize);
                write_ok!(state.result, state.buffer);
                scheduler.handle_coroutine_state(self, state.coroutine)
            }

            State::Write(write_state_ptr) => {
                let mut state = handle_ret_and_get_state!(ret, write_state_ptr, scheduler, self);

                state.buffer.add_offset(ret as usize);
                write_ok!(state.result, state.buffer);
                scheduler.handle_coroutine_state(self, state.coroutine)
            }

            State::WriteAll(write_all_state_ptr) => {
                let mut state = handle_ret_and_get_state!(ret, write_all_state_ptr, scheduler, self);

                if ret as usize == state.buffer.len() {
                    state.buffer.set_offset_to_len();
                    write_ok!(state.result, state.buffer);
                    scheduler.handle_coroutine_state(self, state.coroutine)
                } else {
                    state.buffer.set_offset(state.buffer.offset() + ret as usize);
                    ptr.rewrite_state(scheduler.state_manager().write_all(state.fd, state.buffer, state.offset + ret as usize, state.coroutine, state.result), scheduler.state_manager());

                    self.register(ptr);
                }
            }
        }
    }
}

impl Selector for IoUringSelector {
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
    fn register(&mut self, state_ptr: Ptr<State>) {
        let state = unsafe { state_ptr.as_mut() };

        let mut entry = match state {
            State::Empty(_) => { panic!("[BUG] tried to register an empty state in [`IoUringSelector`]. Please report this issue.") }
            State::AcceptTcp(accept_tcp_state_ptr) => unsafe {
                opcode::Accept::new(types::Fd(accept_tcp_state_ptr.as_ref().fd), ptr::null_mut(), ptr::null_mut())
                    .build()
            }
            State::ConnectTcp(connect_tcp_state_ptr) => unsafe {
                let connect_tcp_state = connect_tcp_state_ptr.as_ref();
                opcode::Connect::new(types::Fd(connect_tcp_state.socket.as_raw_fd()), connect_tcp_state.address.as_ptr(), connect_tcp_state.address.len())
                    .build()
            }
            State::Poll(poll_state_ptr) => unsafe {
                let poll_state = poll_state_ptr.as_ref();
                opcode::PollAdd::new(types::Fd(poll_state.fd), libc::POLLIN as _)
                    .build()
            }
            State::Recv(recv_state_ptr) => unsafe {
                let recv_state = recv_state_ptr.as_mut();
                opcode::Recv::new(types::Fd(recv_state.fd), recv_state.buffer.as_mut_ptr(), recv_state.buffer.cap() as _)
                    .build()
            }
            State::Send(send_state_ptr) => unsafe {
                let send_state = send_state_ptr.as_ref();
                opcode::Send::new(types::Fd(send_state.fd), send_state.buffer.as_ptr(), send_state.buffer.len() as _)
                    .build()
            }
            State::SendAll(send_all_state_ptr) => unsafe {
                let send_all_state = send_all_state_ptr.as_ref();
                opcode::Send::new(types::Fd(send_all_state.fd), send_all_state.buffer.as_ptr(), send_all_state.buffer.len() as _)
                    .build()
            }
            State::Close(close_state_ptr) => unsafe {
                let close_state = close_state_ptr.as_ref();
                opcode::Close::new(types::Fd(close_state.fd))
                    .build()
            }
            State::Open(open_state_ptr) => unsafe {
                todo!();
            }
            State::Read(read_state_ptr) => unsafe {
                let read_state = read_state_ptr.as_mut();
                opcode::Read::new(types::Fd(read_state.fd), read_state.buffer.as_mut_ptr(), read_state.buffer.cap() as _)
                    .offset(read_state.offset as _)
                    .build()
            }
            State::Write(write_state_ptr) => unsafe {
                let write_state = write_state_ptr.as_ref();
                opcode::Write::new(types::Fd(write_state.fd), write_state.buffer.as_ptr(), write_state.buffer.len() as _)
                    .offset(write_state.offset as _)
                    .build()
            }
            State::WriteAll(write_all_state_ptr) => unsafe {
                let write_all_state = write_all_state_ptr.as_ref();
                opcode::Write::new(types::Fd(write_all_state.fd), write_all_state.buffer.as_ptr(), write_all_state.buffer.len() as _)
                    .offset(write_all_state.offset as _)
                    .build()
            }
        };

        entry = entry.user_data(state_ptr.as_u64());
        self.add_sqe(entry);
    }
}