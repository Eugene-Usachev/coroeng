use std::cell::{UnsafeCell};
use std::collections::{BTreeSet, VecDeque};
use std::intrinsics::unlikely;
use std::mem::{MaybeUninit, transmute};
#[allow(unused_imports)] // compiler will complain if it's not used, but we need it for resume()
use std::ops::{Coroutine, CoroutineState};
use std::ptr::null_mut;
use std::time::Instant;
use proc::coro;
use crate::cfg::{config_selector, SelectorType};
use crate::coroutine::coroutine::{CoroutineImpl};
use crate::coroutine::{yield_now, YieldStatus};
use crate::io::sys::unix::{EpolledSelector, IoUringSelector};
use crate::io::{Selector, PollState};
use crate::net::{TcpListener};
use crate::{write_err};
use crate::run::uninit;
use crate::sleep::SleepingCoroutine;
use crate::utils::Ptr;

thread_local! {
    /// [`Scheduler`] for the current thread. It can be uninitialized.
    /// It is initialized in [`init`](Scheduler::init) or [`run_on_core`](crate::run::run_on_core) or [`run_on_all_cores`](crate::run::run_on_all_cores).
    ///
    /// This is thread-local, so it can be used without synchronization.
    pub static LOCAL_SCHEDULER: UnsafeCell<MaybeUninit<Scheduler>> = UnsafeCell::new(MaybeUninit::uninit());
}

/// The scheduler works with coroutines. Specifically, it:
///
/// - saves the coroutines so that they can be woken up later;
///
/// - stores sleeping coroutines and monitors the time until they need to be woken;
///
/// - creates [`Selector`] and works with it.
pub struct Scheduler {
    task_queue: VecDeque<CoroutineImpl>,
    sleeping: BTreeSet<SleepingCoroutine>,

    //blocking_pool: BlockingPool,
    //ready_coroutines: Vec<CoroutineImpl>
}

impl Scheduler {
    /// Initializes the [`Scheduler`] in the [`LOCAL_SCHEDULER`]).
    pub fn init() {
        let scheduler = Self {
            task_queue: VecDeque::with_capacity(8),
            sleeping: BTreeSet::new(),

            //blocking_pool: BlockingPool::new(),
            //ready_coroutines: Vec::with_capacity(8)
        };

        LOCAL_SCHEDULER.with(|local| {
            unsafe {
                *(&mut *local.get()) = MaybeUninit::new(scheduler);
                //(&*local.get()).assume_init_ref().blocking_pool.run();
            };
        });
    }

    /// Uninitializes the [`Scheduler`] in the [`LOCAL_SCHEDULER`]).
    pub fn uninit() {
        LOCAL_SCHEDULER.with(|local| {
            unsafe {
                (&mut *local.get()).assume_init_drop();
            };
        });
    }

    /// Stores the [`coroutine`](CoroutineImpl) in the [`Scheduler`] to wake it up later.
    pub fn sched(&mut self, func: CoroutineImpl) {
        self.task_queue.push_back(func);
    }

    /// Wakes up the sleeping coroutines, which are ready to run.
    pub(crate) fn awake_coroutines(&mut self) {
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

    /// Resume the provided [`coroutine`](CoroutineImpl) and process the result.
    #[inline(always)]
    pub(crate) fn handle_coroutine_state<S: Selector>(&mut self, selector: &mut S, mut task: CoroutineImpl) -> bool {
        let res: CoroutineState<YieldStatus, ()> = task.as_mut().resume(());
        match res {
            CoroutineState::Yielded(status) => {
                match status {
                    YieldStatus::Sleep(dur) => {
                        let sleep = SleepingCoroutine::new(dur, task);
                        self.sleeping.insert(sleep);
                    }

                    YieldStatus::Yield => {
                        self.task_queue.push_back(task);
                    }

                    YieldStatus::End => {
                        return true;
                    }

                    YieldStatus::NewTcpListener(status) => {
                        let fd = TcpListener::get_fd(status.address);
                        unsafe { status.listener_ptr.write(TcpListener::from_fd(fd)); }

                        self.handle_coroutine_state(selector, task);
                    }

                    YieldStatus::TcpConnect(status) => {
                        let state_ = PollState::new_connect_tcp(socket2::SockAddr::from(status.address), task, status.stream_ptr);
                        if state_.is_err() {
                            let (error, task) = unsafe { state_.unwrap_err_unchecked() };
                            write_err!(status.stream_ptr, error);
                            self.handle_coroutine_state(selector, task);
                            return false;
                        }

                        selector.register(unsafe {Ptr::new(state_.unwrap_unchecked())});
                    }

                    YieldStatus::TcpAccept(status) => {
                        let state_ptr = status.state_ref;
                        let state_ref = unsafe { state_ptr.as_ref() };
                        unsafe { state_ptr.write(PollState::new_accept_tcp(state_ref.fd(), task, status.result_ptr)) };
                        if selector.need_reregister() || !status.is_registered {
                            selector.register(state_ptr);
                        }
                    }

                    YieldStatus::TcpRead(status) => {
                        let state_ptr = status.state_ref;
                        let state_ref = unsafe { state_ptr.as_ref() };
                        unsafe { state_ptr.write(PollState::new_poll_tcp(state_ref.fd(), task, status.result_ptr)) };
                        if selector.need_reregister() || !status.is_registered {
                            selector.register(state_ptr);
                        }
                    }

                    YieldStatus::TcpWrite(status) => {
                        let state_ptr = status.state_ref;
                        let state_ref = unsafe { state_ptr.as_ref() };
                        let fd = state_ref.fd();
                        unsafe { state_ptr.write(PollState::new_write_tcp(fd, status.buffer, task, status.result_ptr)) };
                        selector.write(state_ptr);
                    }

                    YieldStatus::TcpWriteAll(status) => {
                        let state_ptr = status.state_ref;
                        let state_ref = unsafe { state_ptr.as_ref() };
                        unsafe { state_ptr.write(PollState::new_write_all_tcp(state_ref.fd(), status.buffer, task, status.result_ptr)) };
                        selector.write_all(state_ptr);
                    }

                    YieldStatus::TcpClose(status) => {
                        let state_ptr = status.state_ptr;
                        let state_ref = unsafe { state_ptr.as_mut() };
                        unsafe { state_ptr.write(PollState::new_close_tcp(state_ref.fd(), task)) };
                        selector.close_connection(state_ptr);
                        //self.handle_coroutine_state(selector, task);
                    }
                }
            }
            CoroutineState::Complete(_) => {}
        }

        false
    }

    // #[inline(always)]
    // fn process_ready_coroutines<S: Selector>(&mut self, selector: &mut S) {
    //     //self.blocking_pool.get_ready(&mut self.ready_coroutines);
    //     let ptr = self.ready_coroutines.as_ptr();
    //     unsafe {
    //         for i in 0. .self.ready_coroutines.len() {
    //             self.handle_coroutine_state(selector, ptr.add(i).read());
    //         }
    //         self.ready_coroutines.set_len(0);
    //     }
    // }

    /// Start the [`Scheduler`] and create [`Selector`].
    pub fn run(&mut self, main_func: CoroutineImpl) {
        match config_selector() {
            SelectorType::Poller => self.run_with_selector(main_func, EpolledSelector::new().expect("Failed to create epoll selector")),
            SelectorType::Ring => self.run_with_selector(main_func, IoUringSelector::new()),
        }
    }

    /// Start the background work. Specifically, it:
    ///
    /// - Awakes sleeping coroutines, which are ready to run.
    ///
    /// - Polls [`Selector`].
    #[coro(crate="crate")]
    fn background_work<S: Selector>(selector_ref: &'static mut S) {
        let scheduler = local_scheduler();
        loop {
            //scheduler.process_ready_coroutines(selector_ref);
            scheduler.awake_coroutines();
            selector_ref.poll(scheduler).expect("Poll error");
            yield yield_now();
        }
    }

    /// Start the [`Scheduler`].
    fn run_with_selector<S: Selector + 'static>(&mut self, main_func: CoroutineImpl, mut selector: S) {
        self.task_queue.push_back(main_func);
        let selector_ref = unsafe { transmute::<&mut S, &'static mut S>(&mut selector) };

        self.sched(Self::background_work(selector_ref, null_mut()));

        let mut task_;
        let mut task;

        loop {
            task_ = self.task_queue.pop_front();

            task = unsafe { task_.unwrap_unchecked() };
            if unlikely(self.handle_coroutine_state(&mut selector, task)) {
                break;
            }
        }

        uninit();
    }
}

/// Returns the [`Scheduler`] from the current thread. Used for low-level work.
pub fn local_scheduler() -> &'static mut Scheduler {
    LOCAL_SCHEDULER.with(|local| {
        unsafe { (&mut *local.get()).assume_init_mut() }
    })
}