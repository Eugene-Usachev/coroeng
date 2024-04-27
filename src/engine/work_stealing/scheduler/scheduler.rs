use std::intrinsics::unlikely;
#[allow(unused_imports)]
use std::ops::{Coroutine as StdCoroutine, CoroutineState};
use std::pin::Pin;
use std::{thread};
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::os::fd::{BorrowedFd};
use std::sync::atomic::AtomicBool;
use core_affinity::CoreId;
use crate::spawn_coroutine;

use crate::engine::coroutine::coroutine::{Coroutine, CoroutineImpl, YieldStatus};
use crate::engine::io::sys::unix::epoll::net;
use crate::engine::io::sys::unix::Selector;
use crate::engine::io::Token;
use crate::engine::net::tcp::TcpListener;
use crate::engine::work_stealing::sleep::global_controller::GlobalController;
use crate::engine::work_stealing::task_queue::spin_lock_task_queue::SpinLockTaskQueue;
use crate::engine::work_stealing::sleep::sleep::SleepingCoroutine;

pub static SCHEDULER: GlobalScheduler = GlobalScheduler::new();
pub static IS_INIT: AtomicBool = AtomicBool::new(false);

pub struct Scheduler {
    task_queue: SpinLockTaskQueue,

    sleep_controller: GlobalController
}

pub struct GlobalScheduler {
    inner: UnsafeCell<MaybeUninit<Scheduler>>,
}

impl GlobalScheduler {
    const fn new() -> Self {
        GlobalScheduler {
            inner: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    pub fn init(&self) {
        unsafe {
            (&mut *self.inner.get()).write(Scheduler::new());
        }
    }

    pub fn sched(&self, func: CoroutineImpl) {
        unsafe {
            (&*self.inner.get()).assume_init_ref().new_coroutine(func);
        }
    }

    pub fn run<Co: Coroutine + 'static>(&self, main_func: Pin<Box<Co>>) {
        unsafe {
            (&*self.inner.get()).assume_init_ref().run(main_func);
        }
    }

    pub fn task_queue(&self) -> &SpinLockTaskQueue {
        unsafe {
            &(&*self.inner.get()).assume_init_ref().task_queue
        }
    }
}

unsafe impl Send for GlobalScheduler {}
unsafe impl Sync for GlobalScheduler {}

impl Scheduler {
    pub fn new() -> Self {

        Scheduler {
            task_queue: SpinLockTaskQueue::new(),

            sleep_controller: GlobalController::new()
        }
    }

    pub fn new_coroutine(&self, func: CoroutineImpl) {
        self.task_queue.push(func);
    }

    fn looping_func(&self, core_id: CoreId) {
        core_affinity::set_for_current(core_id);
        let mut selector = Selector::new(self).expect("failed to create selector");
        let mut task_;
        let mut task;
        let mut res;

        loop {
            task_ = self.task_queue.pop();

            if unlikely(task_.is_none()) {
                selector.poll().expect("Poll error");
                continue;
            }

            task = unsafe { task_.unwrap_unchecked() };
            res = task.as_mut().resume(());
            self.handle_coroutine_state(&mut selector, res, task);
        }
    }

    #[inline(always)]
    pub(crate) fn handle_coroutine_state(&self, selector: &mut Selector, res: CoroutineState<YieldStatus, ()>, mut task: CoroutineImpl) {
        match res {
            CoroutineState::Yielded(status) => {
                match status {
                    YieldStatus::Sleep(dur) => {
                        let sleep = SleepingCoroutine::new(dur, task);
                        self.sleep_controller.add(sleep);
                    }

                    YieldStatus::Yield => {
                        self.task_queue.push(task);
                    }

                    YieldStatus::NewTcpListener(addr, result) => {
                        let fd = TcpListener::get_fd(addr);
                        let token_id = selector.tokens.insert(Token::new_empty(fd));
                        let listener = TcpListener{
                            token_id,
                            fd,
                            is_registered: false
                        };
                        unsafe {*result = listener; }

                        self.handle_coroutine_state(selector, task.as_mut().resume(()), task);
                    }

                    YieldStatus::AcceptTcp(is_registered, listener, result) => {
                        let token = unsafe { selector.tokens.get_unchecked_mut(listener) };
                        *token = Token::new_accept_tcp(token.fd(), task, result);
                        let fd = token.fd();
                        if !is_registered {
                            selector.register(fd, listener);
                        }
                    }

                    YieldStatus::ReadTcp(is_registered, stream, result) => {
                        let token = unsafe { selector.tokens.get_unchecked_mut(stream) };
                        *token = Token::new_read_tcp(token.fd(), task, result);
                        let fd = token.fd();
                        if !is_registered {
                            selector.register(fd, stream);
                        }
                    }

                    YieldStatus::WriteTcp(stream, buf, result) => {
                        let token = unsafe { selector.tokens.get_unchecked_mut(stream) };
                        let fd = token.fd();
                        let res = unsafe {
                            nix::unistd::write(BorrowedFd::borrow_raw(fd), &buf)
                        };
                        if res.is_ok() {
                            unsafe { *result = Ok(res.unwrap_unchecked()) };
                        } else {
                            unsafe { *result = Err("failed to write") };
                        }

                        self.handle_coroutine_state(selector, task.as_mut().resume(()), task);
                    }

                    YieldStatus::CloseTcp(stream) => {
                        let token = selector.tokens.remove(stream);
                        unsafe { net::close_connection(&BorrowedFd::borrow_raw(token.fd()), &selector.epoll) };
                        self.handle_coroutine_state(selector, task.as_mut().resume(()), task);
                    }

                    YieldStatus::Never => {
                        unreachable!("never yield in scheduler")
                    }
                }
            }
            CoroutineState::Complete(_) => {}
        }
    }

    pub fn run<Co: Coroutine + 'static>(&'static self, main_func: Pin<Box<Co>>) {
        let core_ids = core_affinity::get_core_ids().unwrap();

        if core_ids.len() > 1 {
            for i in 1..core_ids.len() {
                let core_id = core_ids[i];
                thread::spawn(move || {
                    self.looping_func(core_id);
                });
            }
        }

        self.task_queue.push(main_func);
        spawn_coroutine!(|| {
            loop {
                self.sleep_controller.awake_coroutines();
                yield YieldStatus::Yield;
            }
        });

        self.looping_func(core_ids[0]);
    }
}