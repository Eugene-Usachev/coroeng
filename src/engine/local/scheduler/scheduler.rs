use std::cell::UnsafeCell;
use std::collections::VecDeque;
use std::intrinsics::unlikely;
use std::mem;
use std::mem::{MaybeUninit, transmute};
use std::ops::{Coroutine, CoroutineState};
use std::os::fd::BorrowedFd;
use std::pin::Pin;
use std::time::Instant;
use core_affinity::CoreId;
use crate::engine::coroutine::coroutine::{CoroutineImpl, YieldStatus};
use crate::engine::io::sys::unix::epoll::net;
use crate::engine::io::sys::unix::{EpolledSelector, IoUringSelector};
use crate::engine::io::{Selector, Token};
use crate::engine::sleep::sleep::SleepingCoroutine;
use crate::utils::buf_pool;

thread_local! {
    pub static LOCAL_SCHEDULER: UnsafeCell<MaybeUninit<Scheduler>> = UnsafeCell::new(MaybeUninit::zeroed());
}

pub struct Scheduler {
    task_queue: VecDeque<CoroutineImpl>,
    core: CoreId,

    sleeping: Vec<SleepingCoroutine>
}

impl Scheduler {
    pub fn init(core: CoreId) {
        let scheduler = Self {
            task_queue: VecDeque::with_capacity(8),
            core,
            sleeping: Vec::with_capacity(8),
        };

        LOCAL_SCHEDULER.with(|local| {
            unsafe { (&mut *local.get()).write(scheduler) };
        });
    }

    pub fn sched(&mut self, func: CoroutineImpl) {
        self.task_queue.push_back(func);
    }

    pub fn awake_coroutines(&mut self) {
        let mut new_list = Vec::with_capacity(self.sleeping.len());
        mem::swap(&mut new_list, &mut self.sleeping);
        let now = Instant::now();
        for sleeping_coroutine in new_list.into_iter() {
            if unlikely(now >= sleeping_coroutine.execution_time) {
                self.sched(sleeping_coroutine.co);
                continue;
            }
            self.sleeping.push(sleeping_coroutine)
        }
    }

    #[inline(always)]
    pub(crate) fn handle_coroutine_state<S: Selector>(&mut self, selector: &mut S, mut task: CoroutineImpl) {
        let res = task.as_mut().resume(());
        match res {
            CoroutineState::Yielded(status) => {
                match status {
                    YieldStatus::Sleep(dur) => {
                        let sleep = SleepingCoroutine::new(dur, task);
                        self.sleeping.push(sleep);
                    }

                    YieldStatus::Yield => {
                        self.task_queue.push_back(task);
                    }

                    YieldStatus::NewTcpListener(addr, result) => {
                        let fd = crate::engine::net::tcp::TcpListener::get_fd(addr);
                        let token_id = selector.insert_token(Token::new_empty(fd));
                        let listener = crate::engine::net::tcp::TcpListener {
                            token_id,
                            fd,
                            is_registered: false
                        };
                        unsafe {*result = listener; }

                        self.handle_coroutine_state(selector, task);
                    }

                    YieldStatus::TcpAccept(is_registered, listener, result) => {
                        let token = selector.get_token_mut_ref(listener);
                        *token = Token::new_accept_tcp(token.fd(), task, result);
                        if selector.need_reregister() || !is_registered {
                            selector.register(listener);
                        }
                    }

                    YieldStatus::TcpRead(is_registered, stream, result) => {
                        let token =  selector.get_token_mut_ref(stream);
                        *token = Token::new_poll_tcp(token.fd(), task, result);
                        if selector.need_reregister() || !is_registered {
                            selector.register(stream);
                        }
                    }

                    YieldStatus::TcpWrite(stream, buf, result) => {
                        let token = selector.get_token_mut_ref(stream);
                        *token = Token::new_write_tcp(token.fd(), buf, task, result);
                        selector.write(stream);
                    }

                    YieldStatus::TcpWriteAll(stream, buf, result) => {
                        let token = selector.get_token_mut_ref(stream);
                        *token = Token::new_write_all_tcp(token.fd(), buf, task, result);
                        selector.write_all(stream);
                    }

                    YieldStatus::TcpClose(socket) => {
                        let token = selector.get_token_mut_ref(socket);
                        *token = Token::new_close_tcp(token.fd(), task);
                        selector.close_connection(socket);
                    }

                    YieldStatus::Never => {
                        unreachable!("never yield in scheduler")
                    }
                }
            }
            CoroutineState::Complete(_) => {}
        }
    }

    pub fn run<Co: Coroutine<Yield = YieldStatus, Return=()> + 'static>(&mut self, main_func: Pin<Box<Co>>) {
        self.task_queue.push_back(main_func);
        let mut selector = EpolledSelector::new().expect("failed to create selector");
        //let mut selector = IoUringSelector::new();
        let selector_ref = unsafe { transmute::<&mut EpolledSelector, &'static mut EpolledSelector>(&mut selector) };
        //let selector_ref = unsafe { transmute::<&mut IoUringSelector, &'static mut IoUringSelector>(&mut selector) };

        self.sched(Box::pin(#[coroutine] || {
            let scheduler = local_scheduler();
            loop {
                scheduler.awake_coroutines();
                selector_ref.poll(scheduler).expect("Poll error");
                yield YieldStatus::Yield;
            }
        }));

        let mut task_;
        let mut task;

        loop {
            task_ = self.task_queue.pop_front();

            task = unsafe { task_.unwrap_unchecked() };
            self.handle_coroutine_state(&mut selector, task);
        }
    }
}

pub fn local_scheduler() -> &'static mut Scheduler {
    LOCAL_SCHEDULER.with(|local| {
        unsafe { (&mut *local.get()).assume_init_mut() }
    })
}