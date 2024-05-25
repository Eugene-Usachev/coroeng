use std::cell::{UnsafeCell};
use std::collections::{BTreeSet, VecDeque};
use std::intrinsics::{likely, unlikely};
use std::mem;
use std::mem::{MaybeUninit, transmute};
#[allow(unused_imports)] // compiler will complain if it's not used, but we need it for resume()
use std::ops::{Coroutine, CoroutineState};
use std::time::Instant;
use crate::cfg::{config_selector, SelectorType};
use crate::coroutine::coroutine::{CoroutineImpl};
use crate::coroutine::YieldStatus;
use crate::io::sys::unix::{EpolledSelector, IoUringSelector};
use crate::io::{Selector, State};
use crate::net::TcpListener;
use crate::new_coroutine;
use crate::sleep::SleepingCoroutine;

thread_local! {
    pub static LOCAL_SCHEDULER: UnsafeCell<MaybeUninit<Scheduler>> = UnsafeCell::new(MaybeUninit::zeroed());
}

// TODO docs
pub struct Scheduler {
    task_queue: VecDeque<CoroutineImpl>,

    sleeping: BTreeSet<SleepingCoroutine>,
    need_to_wake: Vec<Instant>
}

impl Scheduler {
    pub fn init() {
        let scheduler = Self {
            task_queue: VecDeque::with_capacity(8),

            sleeping: BTreeSet::new(),
            need_to_wake: Vec::with_capacity(8)
        };

        LOCAL_SCHEDULER.with(|local| {
            unsafe { (&mut *local.get()).write(scheduler) };
        });
    }

    pub fn sched(&mut self, func: CoroutineImpl) {
        self.task_queue.push_back(func);
    }

    pub fn awake_coroutines(&mut self) {
        let now = Instant::now();
        loop {
            if let Some(sleeping_coroutine) = self.sleeping.pop_first() {
                if now >= sleeping_coroutine.execution_time {
                    self.sched(sleeping_coroutine.co);
                } else {
                    self.sleeping.insert(sleeping_coroutine);
                    break;
                }
            } else {
                break;
            }
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
                        self.sleeping.insert(sleep);
                        println!("sleeping: {}", self.sleeping.len());
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
                        let state_ptr = status.state_ref;
                        let state_ref = unsafe { state_ptr.as_ref() };
                        unsafe { state_ptr.write(State::new_accept_tcp(state_ref.fd(), task, status.result_ptr)) };
                        if selector.need_reregister() || !status.is_registered {
                            selector.register(state_ptr);
                        }
                    }

                    YieldStatus::TcpRead(status) => {
                        let state_ptr = status.state_ref;
                        let state_ref = unsafe { state_ptr.as_ref() };
                        unsafe { state_ptr.write(State::new_poll_tcp(state_ref.fd(), task, status.result_ptr)) };
                        if selector.need_reregister() || !status.is_registered {
                            selector.register(state_ptr);
                        }
                    }

                    YieldStatus::TcpWrite(status) => {
                        let state_ptr = status.state_ref;
                        let state_ref = unsafe { state_ptr.as_ref() };
                        let fd = state_ref.fd();
                        unsafe { state_ptr.write(State::new_write_tcp(fd, status.buffer, task, status.result_ptr)) };
                        selector.write(state_ptr);
                    }

                    YieldStatus::TcpWriteAll(status) => {
                        let state_ptr = status.state_ref;
                        let state_ref = unsafe { state_ptr.as_ref() };
                        unsafe { state_ptr.write(State::new_write_all_tcp(state_ref.fd(), status.buffer, task, status.result_ptr)) };
                        selector.write_all(state_ptr);
                    }

                    YieldStatus::TcpClose(status) => {
                        let state_ptr = status.state_ptr;
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

    pub fn run(&mut self, main_func: CoroutineImpl) {
        match config_selector() {
            SelectorType::Poller => self.run_with_selector(main_func, EpolledSelector::new().expect("Failed to create epoll selector")),
            SelectorType::Ring => self.run_with_selector(main_func, IoUringSelector::new()),
        }
    }

    fn run_with_selector<S: Selector + 'static>(&mut self, main_func: CoroutineImpl, mut selector: S) {
        self.task_queue.push_back(main_func);
        let selector_ref = unsafe { transmute::<&mut S, &'static mut S>(&mut selector) };

        self.sched(new_coroutine!({
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