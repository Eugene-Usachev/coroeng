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
use crate::coroutine::{end, yield_now, YieldStatus};
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
///
/// # Why LIFO?
///
/// The FIFO approach ensures fairness. However, our goal is not to achieve fairness, but to maximize performance.
/// The LIFO approach is more efficient because it allows us to take advantage of data from previous tasks.
/// This is because the data is already stored in the processor cache (by the parent coroutine), so we can use it more effectively.
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
    /// Use [`spawn_local`](crate::spawn_local) instead if you don't want to low-level work.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use std::ptr::null_mut;
    /// use engine::{coro, local_scheduler};
    ///
    /// #[coro]
    /// fn say_hello(from: &str) {
    ///     println!("hello, world from {:?} !", from)
    /// }
    ///
    /// local_scheduler().sched(say_hello("sched method", null_mut()));
    /// ```
    pub fn sched(&mut self, func: CoroutineImpl) {
        self.task_queue.push_back(func);
    }

    /// Wakes up the sleeping coroutines, which are ready to run.
    ///
    /// # Return
    ///
    /// Returns true if [`end`](YieldStatus::End) was handled.
    pub(crate) fn awake_coroutines<S: Selector>(&mut self, selector: &mut S) -> bool {
        let now = Instant::now();
        loop {
            if let Some(sleeping_coroutine) = self.sleeping.pop_first() {
                if now >= sleeping_coroutine.execution_time {
                    if unlikely(self.handle_coroutine_state(selector, sleeping_coroutine.co)) {
                        return true;
                    }
                } else {
                    self.sleeping.insert(sleeping_coroutine);
                    break;
                }
            } else {
                break;
            }
        }

        false
    }

    /// Resume the provided [`coroutine`](CoroutineImpl) and process the result.
    ///
    /// # Return
    ///
    /// Returns true if [`end`](YieldStatus::End) was handled.
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
                        self.task_queue.push_front(task);
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
            if unlikely(scheduler.awake_coroutines(selector_ref)) {
                yield end();
            }

            if unlikely(selector_ref.poll(scheduler).expect("Poll error")) {
                yield end();
            }

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
            task_ = self.task_queue.pop_back();

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

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use super::*;
    use crate::{test_local, coro, sleep::sleep};
    use crate::local::Local;

    #[test_local(crate="crate")]
    fn test_sched() {
        #[coro(crate="crate")]
        fn insert(number: u16, arr: Local<Vec<u16>>) {
            arr.get_mut().push(number);
        }

        let scheduler = local_scheduler();
        let arr = Local::new(Vec::new());

        arr.get_mut().push(10);
        // 30 because LIFO
        scheduler.sched(insert(30, arr.clone(), null_mut()));
        scheduler.sched(insert(20, arr.clone(), null_mut()));

        yield yield_now();

        assert_eq!(&vec![10, 20, 30], arr.get());
    }

    #[test_local(crate="crate")]
    fn test_sleep() {
        #[coro(crate="crate")]
        fn sleep_for(dur: Duration, number: u16, arr: Local<Vec<u16>>) {
            yield sleep(dur);
            arr.get_mut().push(number);
        }

        let scheduler = local_scheduler();
        let arr = Local::new(Vec::new());
        scheduler.sched(sleep_for(Duration::from_millis(1), 1, arr.clone(), null_mut()));
        scheduler.sched(sleep_for(Duration::from_millis(4), 4, arr.clone(), null_mut()));
        scheduler.sched(sleep_for(Duration::from_millis(3), 3, arr.clone(), null_mut()));
        scheduler.sched(sleep_for(Duration::from_millis(2), 2, arr.clone(), null_mut()));

        yield sleep(Duration::from_millis(5));
        assert_eq!(&vec![1, 2, 3, 4], arr.get());
    }
}