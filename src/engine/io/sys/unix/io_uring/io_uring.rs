use std::collections::VecDeque;
use std::io::Error;
use std::os::fd::RawFd;
use std::{mem, ptr};
use io_uring::{cqueue, IoUring, opcode, squeue, types};
use slab::Slab;
use crate::engine::io::{Selector, Token};
use crate::engine::local::Scheduler;
use crate::engine::net::TcpStream;
use crate::utils;
use crate::utils::Buffer;

const MAX_BACKLOG_BEFORE_FLUSH: usize = 10;

pub(crate) struct IoUringSelector {
    tokens: Slab<Token>,
    ring: IoUring<squeue::Entry, cqueue::Entry>,
    backlog: VecDeque<squeue::Entry>
}

impl IoUringSelector {
    pub(crate) fn new() -> IoUringSelector {
        IoUringSelector {
            tokens: Slab::new(),
            ring: IoUring::builder().build(512).unwrap(),
            backlog: VecDeque::with_capacity(MAX_BACKLOG_BEFORE_FLUSH)
        }
    }

    fn flush_backlog(&mut self) {
        todo!()
        // let mut sq = self.ring.submission();
        // let mut vacant = sq.capacity() - sq.len();
        //
        // loop {
        //     if vacant == 0 {
        //         match self.ring.submit() {
        //             Ok(_) => (),
        //             Err(ref err) if err.raw_os_error() == Some(libc::EBUSY) => return,
        //             Err(err) => {
        //                 panic!("IoUringSelector: failed to submit: {}", err);
        //             },
        //         }
        //         vacant = sq.capacity();
        //     }
        //     match self.backlog.pop_front() {
        //         Some(sqe) => unsafe {
        //             let _ = sq.push(&sqe);
        //             vacant -= 1;
        //         },
        //         None => break,
        //     }
        // }
        //
        // sq.sync();
    }

    fn flush_backlog_if_necessary(&mut self) {
        if self.backlog.len() >= MAX_BACKLOG_BEFORE_FLUSH {
            self.flush_backlog();
        }
    }

    fn push_sqe(&mut self, sqe: squeue::Entry) {
        unsafe {
            if self.ring.submission().push(&sqe).is_err() {
                self.backlog.push_back(sqe);
            }
        }
    }
}

macro_rules! handle_ret {
    ($ret: expr, $result: expr) => {
        if ret < 0 {
            let err = Error::last_os_error();
            unsafe {
                *$result = Err(err);
            }
        }
    };
}

impl Selector for IoUringSelector {
    fn insert_token(&mut self, token: Token) -> usize {
        self.tokens.insert(token)
    }

    #[inline(always)]
    fn get_token_mut_ref(&mut self, token_id: usize) -> &mut Token {
        unsafe { self.tokens.get_unchecked_mut(token_id) }
    }

    fn poll(&mut self, scheduler: &mut Scheduler) -> Result<(), ()> {
        todo!()
        // let (submitter, mut sq, mut cq) = self.ring.split();
        //
        // // clean backlog
        // loop {
        //     if sq.is_full() {
        //         match submitter.submit() {
        //             Ok(_) => (),
        //             Err(ref err) if err.raw_os_error() == Some(libc::EBUSY) => break,
        //             Err(err) => {
        //                 panic!("IoUringSelector: failed to submit: {}", err);
        //             },
        //         }
        //     }
        //     sq.sync();
        //
        //     match self.backlog.pop_front() {
        //         Some(sqe) => unsafe {
        //             let _ = sq.push(&sqe);
        //         },
        //         None => break,
        //     }
        // }
        //
        // // TODO: do we need to wait here
        // // match submitter.submit_and_wait(1) {
        // //     Ok(_) => (),
        // //     Err(ref err) if err.raw_os_error() == Some(libc::EBUSY) => (),
        // //     Err(err) => return Err(err.into()),
        // // }
        // cq.sync();
        //
        // for cqe in &mut cq {
        //     let ret = cqe.result();
        //     let token_index = cqe.user_data() as usize;
        //     let Some(token_ref) = self.tokens.get_mut(token_index) else {
        //         continue;
        //     };
        //
        //     let mut token = Token::Empty(token_ref.fd());
        //     mem::swap(token_ref, &mut token);
        //
        //     match token {
        //         Token::Empty(_) => {}
        //         Token::AcceptTcp(_listener_fd, coroutine, result) => {
        //             handle_ret!(ret, result);
        //
        //             let incoming_fd = ret;
        //             let token_id = self.insert_token(Token::Empty(incoming_fd));
        //             unsafe {
        //                 *result = Ok(TcpStream::new(token_id));
        //             }
        //
        //             scheduler.handle_coroutine_state(self, coroutine);
        //         }
        //         Token::PollTcp(fd, coroutine, result) => {
        //             handle_ret!(ret, result);
        //
        //             let buffer = utils::buffer();
        //             *token_ref = Token::new_read_tcp(fd, buffer, coroutine, result);
        //             self.register(fd, token_index);
        //         }
        //         Token::ReadTcp(_stream_fd, buffer, coroutine, result) => {
        //             handle_ret!(ret, result);
        //
        //             unsafe {
        //                 *result = Ok(buffer.as_slice());
        //             }
        //
        //             scheduler.handle_coroutine_state(self, coroutine);
        //         }
        //
        //         Token::WriteTcp(_stream_fd, buffer, coroutine, result) => {
        //             handle_ret!(ret, result);
        //             buffer.release();
        //             scheduler.handle_coroutine_state(self, coroutine)
        //         }
        //     }
        // }
        //
        // Ok(())
    }

    fn register(&mut self, fd: RawFd, token_id: usize) {
        todo!()
        // let token = unsafe { self.tokens.get_unchecked_mut(token_id) };
        // let sqe = match token {
        //     Token::Empty(_) => {
        //         return;
        //
        //     }
        //     Token::AcceptTcp(_, _, _) => {
        //         opcode::Accept::new(types::Fd(fd), ptr::null_mut(), ptr::null_mut())
        //             .build()
        //             .user_data(token_id as _)
        //     }
        //
        //     Token::PollTcp(_, _, _) => {
        //         opcode::PollAdd::new(types::Fd(fd), libc::POLLIN as _)
        //             .build()
        //             .user_data(token_id as _)
        //     }
        //
        //     Token::ReadTcp(_, buffer, _, _) => {
        //         opcode::Recv::new(types::Fd(fd), buffer.as_mut_ptr(), buffer.len() as u32)
        //             .build()
        //             .user_data(token_id as _)
        //     }
        //
        //     Token::WriteTcp(_, buffer, _, _) => {
        //         opcode::Send::new(types::Fd(fd), buffer.as_ptr(), buffer.len() as u32)
        //             .build()
        //             .user_data(token_id as _)
        //     }
        // };
        //
        // self.push_sqe(sqe);
    }

    fn deregister(&mut self, token_id: usize) -> Token {
        todo!()
    }

    fn write(&mut self, token_id: usize) {
        todo!()
        //let token = unsafe { self.tokens.get_unchecked_mut(token_id) };
        //*token = Token::WriteTcp(token.fd(), buffer, token, 0);
    }

    fn write_all(&mut self, token_id: usize) {
        todo!()
    }

    fn close_connection(token: Token) {
        todo!()
    }
}