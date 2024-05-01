use std::collections::VecDeque;
use std::io::Error;
use std::{mem, ptr};
use std::cell::UnsafeCell;
use io_uring::{cqueue, IoUring, opcode, squeue, SubmissionQueue, CompletionQueue, Submitter, types};
use io_uring::types::{SubmitArgs, Timespec};
use slab::Slab;
use crate::engine::io::{Selector, Token};
use crate::engine::local::Scheduler;
use crate::engine::net::TcpStream;
use crate::{utils, write_ok};
use crate::utils::hide_mut_unsafe;

pub(crate) struct IoUringSelector {
    tokens: Slab<Token>,
    timeout: SubmitArgs<'static, 'static>,
    /// # Why we need some cell?
    /// 
    /// We can't rewrite engine ([`Selector`] trait) to use only `tokens` field.
    /// And we can't use `&mut self` in [`Scheduler::handle_coroutine_state`] function, because we are borrowing the `ring` before [`Scheduler::handle_coroutine_state`].
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
            tokens: Slab::new(),
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
            unsafe {
                *$result = Err(err);
            }
            continue;
        }
    };
}

impl Selector for IoUringSelector {
    #[inline(always)]
    fn need_reregister(&self) -> bool {
        true
    }

    fn insert_token(&mut self, token: Token) -> usize {
        self.tokens.insert(token)
    }

    #[inline(always)]
    fn get_token_mut_ref(&mut self, token_id: usize) -> &mut Token {
        unsafe { self.tokens.get_unchecked_mut(token_id) }
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
            let token_index = cqe.user_data() as usize;
            let token_ref = unsafe { self.tokens.get_unchecked_mut(token_index) };

            let mut token = Token::new_empty(token_ref.fd());
            mem::swap(token_ref, &mut token);

            println!("Handle token: {token:?} with ret: {ret}");

            match token {
                Token::Empty(_) => {}

                Token::AcceptTcp(token) => {
                    handle_ret!(ret, token.result);

                    let incoming_fd = ret;
                    let token_id = self.insert_token(Token::new_empty(incoming_fd));
                    write_ok!(token.result, TcpStream::new(token_id));

                    scheduler.handle_coroutine_state(self, token.coroutine);
                }

                Token::PollTcp(token) => {
                    handle_ret!(ret, token.result);

                    println!("poll ret: {}", ret);

                    let buffer = utils::buffer();
                    *token_ref = Token::new_read_tcp(token.fd, buffer, token.coroutine, token.result);
                    self.register(token_index);
                }

                Token::ReadTcp(token) => {
                    println!("read ret: {}", ret);

                    handle_ret!(ret, token.result);

                    write_ok!(token.result, mem::transmute(token.buffer.as_slice()));

                    scheduler.handle_coroutine_state(self, token.coroutine);
                }

                Token::WriteTcp(token) => {
                    handle_ret!(ret, token.result);

                    write_ok!(token.result, ret as usize);

                    scheduler.handle_coroutine_state(self, token.coroutine)
                }

                Token::WriteAllTcp(mut token) => {
                    handle_ret!(ret, token.result);
                    println!("write all ret: {}", ret);
                    let was_written = ret as usize;
                    if token.bytes_written + was_written < token.buffer.len() {
                        token.bytes_written += was_written;
                        let sqe = opcode::Write::new(
                            types::Fd(token.fd),
                            unsafe { token.buffer.as_ptr().add(token.bytes_written) },
                            (token.buffer.len() - token.bytes_written) as u32
                        ).build().user_data(token_index as _);
                        *token_ref = Token::WriteAllTcp(token);
                        self.push_sqe(sqe);
                        continue;
                    }

                    write_ok!(token.result, ());

                    scheduler.handle_coroutine_state(self, token.coroutine)
                }

                Token::CloseTcp(token) => {
                    self.deregister(token_index);
                    scheduler.handle_coroutine_state(self, token.coroutine)
                }
            }
        }

        Ok(())
    }

    fn register(&mut self, token_id: usize) {
        let token = unsafe { self.tokens.get_unchecked_mut(token_id) };
        let mut sqe = match token {
            Token::Empty(_) => {
                return;
            }

            Token::AcceptTcp(token) => {
                opcode::Accept::new(types::Fd(token.fd), ptr::null_mut(), ptr::null_mut())
                    .build()
            }

            Token::PollTcp(token) => {
                opcode::PollAdd::new(types::Fd(token.fd), libc::POLLIN as _)
                    .build()
            }

            Token::ReadTcp(token) => {
                opcode::Recv::new(types::Fd(token.fd), token.buffer.as_mut_ptr(), token.buffer.cap() as u32)
                    .build()
            }

            Token::WriteTcp(token) => {
                opcode::SendZc::new(types::Fd(token.fd), token.buffer.as_ptr(), token.buffer.len() as u32)
                    .build()
            }

            Token::WriteAllTcp(token) => {
                // TODO: maybe we need to use token.buffer.as_ptr().add(token.bytes_written) and use register in poll?
                opcode::SendZc::new(types::Fd(token.fd), token.buffer.as_ptr(), token.buffer.len() as u32)
                    .build()
            }

            Token::CloseTcp(token) => {
                opcode::Close::new(types::Fd(token.fd))
                    .build()
            }
        };

        sqe = sqe.user_data(token_id as _);

        println!("register sqe: {:?} for token id: {token_id}, sqe: {sqe:?}", token);
        self.push_sqe(sqe);
    }

    fn deregister(&mut self, token_id: usize) -> Token {
        self.tokens.remove(token_id)
    }

    fn write(&mut self, token_id: usize) {
        self.register(token_id);
    }

    fn write_all(&mut self, token_id: usize) {
        self.register(token_id);
    }

    fn close_connection(&mut self, token_id: usize) {
        self.register(token_id);
    }
}