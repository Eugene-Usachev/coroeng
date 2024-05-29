use std::thread;
use crate::coroutine::CoroutineImpl;
use crate::io::BlockingState;
use crate::scheduler::blocking_pool::worker::Worker;

pub(crate) struct BlockingPool {
    // TODO try many workers
    worker: Worker
}

impl BlockingPool {
    pub(crate) fn new() -> Self {
        let worker = Worker::new();
        let pool = Self {
            worker
        };

        pool
    }

    pub(crate) fn run(&self) {
        let worker_ref = unsafe { std::mem::transmute::<&Worker, &'static Worker>(&self.worker) };
        thread::spawn(move || {
            worker_ref.run();
        });
    }

    #[inline(always)]
    pub(crate) fn get_ready(&self, ready: &mut Vec<CoroutineImpl>) {
        self.worker.get_ready(ready);
    }

    #[inline(always)]
    pub(crate) fn put_state(&self, state: BlockingState) {
        self.worker.put_state(state);
    }
}