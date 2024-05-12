use std::cell::UnsafeCell;
use std::collections::VecDeque;
use std::intrinsics::{size_of_val, unlikely};
use std::mem;
use std::mem::{MaybeUninit, transmute};
use std::ops::{Coroutine, CoroutineState};
use std::pin::Pin;
use std::ptr::NonNull;
use std::time::Instant;
use core_affinity::CoreId;
use crate::engine::coroutine::coroutine::{CoroutineImpl};
use crate::engine::coroutine::YieldStatus;
use crate::engine::io::sys::unix::{EpolledSelector,};
use crate::engine::io::{Selector, State};
use crate::engine::net::TcpListener;
use crate::engine::sleep::sleep::SleepingCoroutine;
use crate::utils::{hide_mut_unsafe, Ptr};

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
        let res: CoroutineState<YieldStatus, ()> = task.as_mut().resume(());
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

                    YieldStatus::NewTcpListener(status) => {
                        let fd = TcpListener::get_fd(status.address);
                        unsafe { status.listener_ptr.write(TcpListener::from_fd(fd)); }

                        self.handle_coroutine_state(selector, task);
                    }

                    YieldStatus::TcpAccept(status) => {
                        let mut state_ptr = status.state_ref;
                        let state_ref = unsafe { state_ptr.as_ref() };
                        unsafe { state_ptr.write(State::new_accept_tcp(state_ref.fd(), task, status.result_ptr)) };
                        if selector.need_reregister() || !status.is_registered {
                            selector.register(state_ptr);
                        }
                    }

                    YieldStatus::TcpRead(status) => {
                        let mut state_ptr = status.state_ref;
                        let state_ref = unsafe { state_ptr.as_ref() };
                        unsafe { state_ptr.write(State::new_poll_tcp(state_ref.fd(), task, status.result_ptr)) };
                        if selector.need_reregister() || !status.is_registered {
                            selector.register(state_ptr);
                        }
                    }

                    YieldStatus::TcpWrite(status) => {
                        let mut state_ptr = status.state_ref;
                        let state_ref = unsafe { state_ptr.as_ref() };
                        let fd = state_ref.fd();
                        unsafe { state_ptr.write(State::new_write_tcp(fd, status.buffer, task, status.result_ptr)) };
                        selector.write(state_ptr);
                    }

                    YieldStatus::TcpWriteAll(status) => {
                        let mut state_ptr = status.state_ref;
                        let state_ref = unsafe { state_ptr.as_ref() };
                        unsafe { state_ptr.write(State::new_write_all_tcp(state_ref.fd(), status.buffer, task, status.result_ptr)) };
                        selector.write_all(state_ptr);
                    }

                    YieldStatus::TcpClose(status) => {
                        let mut state_ptr = status.state_ptr;
                        let state_ref = unsafe { state_ptr.as_mut() };
                        unsafe { state_ptr.write(State::new_close_tcp(state_ref.fd(), task)) };
                        selector.close_connection(state_ptr);
                        //self.handle_coroutine_state(selector, task);
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