use std::collections::VecDeque;
use std::io::Error;
use std::{mem, ptr};
use std::cell::UnsafeCell;
use std::os::fd::RawFd;
use io_uring::{cqueue, IoUring, opcode, squeue, SubmissionQueue, CompletionQueue, Submitter, types};
use io_uring::types::{SubmitArgs, Timespec};
use crate::engine::io::{Selector, State};
use crate::engine::local::Scheduler;
use crate::engine::net::TcpStream;
use crate::{utils, write_ok};
use crate::utils::{hide_mut_unsafe, Ptr};

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

const TIMEOUT: Timespec = Timespec::new().nsec(500_000);

impl IoUringSelector {
    // FIXME: remove unsafe after [`IoUringSelector`] fixed
    pub(crate) unsafe fn new() -> IoUringSelector {
        let mut selector = IoUringSelector {
            timeout: SubmitArgs::new().timespec(&TIMEOUT),
            ring: UnsafeCell::new(IoUring::new(512).unwrap()),
            /// Should be enough for anybody, because it uses only when [`SubmissionQueue`] is full.
            ///
            /// Every sqe is 64 bytes, so 64 * 64 bytes = 4 KB.
            backlog: VecDeque::with_capacity(64)
        };

        selector
    }

    #[inline(always)]
    fn flush(
        backlog: &mut VecDeque<squeue::Entry>,
        submitter: &mut Submitter,
        sq: &mut SubmissionQueue<squeue::Entry>
    ) {
        println!("flush len: {}", sq.len());
        let mut vacant = sq.capacity() - sq.len();

        loop {
            if vacant == 0 {
                match submitter.submit() {
                    Ok(_) => (),
                    Err(ref err) if err.raw_os_error() == Some(libc::EBUSY) => break,
                    Err(err) => {
                        panic!("IoUringSelector: failed to submit: {}", err);
                    },
                }
                vacant = sq.capacity();
                sq.sync();
            }
            match backlog.pop_front() {
                Some(sqe) => unsafe {
                    let _ = sq.push(&sqe);
                    vacant -= 1;
                },
                None => break,
            }
        }

        sq.sync();
    }

    fn push_sqe(&mut self, sqe: squeue::Entry) {
        unsafe {
            if hide_mut_unsafe(&self.ring).submission().push(&sqe).is_err() {
                self.backlog.push_back(sqe);
            }
        }
        println!("now len: {}", hide_mut_unsafe(&self.ring).submission().len());
    }
}

macro_rules! handle_ret {
    ($ret: expr, $result: expr) => {
        if $ret < 0 {
            let err = Error::last_os_error();
            unsafe { $result.write(Err(err)); }
            continue;
        }
    };
}

impl Selector for IoUringSelector {
    #[inline(always)]
    fn need_reregister(&self) -> bool {
        true
    }

    fn poll(&mut self, scheduler: &mut Scheduler) -> Result<(), ()> {
        let (mut submitter, mut sq, mut cq) = hide_mut_unsafe(&self.ring).split();
        match submitter.submit_with_args(1, &self.timeout) {
            Ok(_) => (),
            Err(ref err) if err.raw_os_error() == Some(libc::ETIME) => (),
            Err(ref err) if err.raw_os_error() == Some(libc::EBUSY) => (),
            Err(_err) => {
                return Err(());
            },
        }
        cq.sync();

        loop {
            if sq.is_full() {
                match submitter.submit() {
                    Ok(_) => (),
                    Err(ref err) if err.raw_os_error() == Some(libc::EBUSY) => break,
                    Err(_err) => return Err(()),
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

        for cqe in &mut cq {
            let ret = cqe.result();
            let state_ptr = Ptr::from(cqe.user_data() as usize);
            let state = unsafe { state_ptr.read() };

            println!("Handle state: {:?} with ret: {ret}", unsafe { state_ptr.as_ref() });

            match state {
                State::Empty(_) => {}

                State::AcceptTcp(state) => {
                    handle_ret!(ret, state.result);

                    let incoming_fd = ret;
                    write_ok!(state.result, TcpStream::new(incoming_fd));

                    scheduler.handle_coroutine_state(self, state.coroutine);
                }

                State::PollTcp(state) => {
                    handle_ret!(ret, state.result);

                    println!("poll ret: {}", ret);

                    let buffer = utils::buffer();
                    unsafe { state_ptr.write(State::new_read_tcp(state.fd, buffer, state.coroutine, state.result)) };
                    self.register(state_ptr);
                }

                State::ReadTcp(state) => {
                    println!("read ret: {}", ret);

                    handle_ret!(ret, state.result);

                    write_ok!(state.result, mem::transmute(state.buffer.as_slice()));

                    scheduler.handle_coroutine_state(self, state.coroutine);
                }

                State::WriteTcp(state) => {
                    handle_ret!(ret, state.result);

                    write_ok!(state.result, ret as usize);

                    scheduler.handle_coroutine_state(self, state.coroutine)
                }

                State::WriteAllTcp(mut state) => {
                    handle_ret!(ret, state.result);
                    println!("write all ret: {}", ret);
                    let was_written = ret as usize;
                    if state.bytes_written + was_written < state.buffer.len() {
                        state.bytes_written += was_written;
                        let sqe = opcode::Write::new(
                            types::Fd(state.fd),
                            unsafe { state.buffer.as_ptr().add(state.bytes_written) },
                            (state.buffer.len() - state.bytes_written) as u32
                        ).build().user_data(state_ptr.as_u64());
                        unsafe { state_ptr.write(State::WriteAllTcp(state)) };
                        self.push_sqe(sqe);
                        continue;
                    }

                    write_ok!(state.result, ());

                    scheduler.handle_coroutine_state(self, state.coroutine)
                }

                State::CloseTcp(state) => {
                    self.deregister(state.fd);
                    scheduler.handle_coroutine_state(self, state.coroutine)
                }
            }
        }

        Ok(())
    }

    fn register(&mut self, state_ptr: Ptr<State>) {
        let state = unsafe { state_ptr.as_mut() };
        let mut sqe = match state {
            State::Empty(_) => {
                return;
            }

            State::AcceptTcp(state) => {
                opcode::Accept::new(types::Fd(state.fd), ptr::null_mut(), ptr::null_mut())
                    .build()
            }

            State::PollTcp(state) => {
                opcode::PollAdd::new(types::Fd(state.fd), libc::POLLIN as _)
                    .build()
            }

            State::ReadTcp(state) => {
                opcode::Recv::new(types::Fd(state.fd), state.buffer.as_mut_ptr(), state.buffer.cap() as u32)
                    .build()
            }

            State::WriteTcp(state) => {
                opcode::SendZc::new(types::Fd(state.fd), state.buffer.as_ptr(), state.buffer.len() as u32)
                    .build()
            }

            State::WriteAllTcp(state) => {
                // TODO: maybe we need to use state.buffer.as_ptr().add(state.bytes_written) and use register in poll?
                opcode::SendZc::new(types::Fd(state.fd), state.buffer.as_ptr(), state.buffer.len() as u32)
                    .build()
            }

            State::CloseTcp(state) => {
                opcode::Close::new(types::Fd(state.fd))
                    .build()
            }
        };

        sqe = sqe.user_data(state_ptr.as_u64());

        //println!("register sqe: {:?} for state id: {state_id}, sqe: {sqe:?}", state);
        self.push_sqe(sqe);
    }

    #[inline(always)]
    fn deregister(&mut self, fd: RawFd) {}

    #[inline(always)]
    fn write(&mut self, state_ref: Ptr<State>) {
        self.register(state_ref);
    }

    #[inline(always)]
    fn write_all(&mut self, state_ref: Ptr<State>) {
        self.register(state_ref);
    }

    #[inline(always)]
    fn close_connection(&mut self, state_ref: Ptr<State>) {
        self.register(state_ref);
    }
}