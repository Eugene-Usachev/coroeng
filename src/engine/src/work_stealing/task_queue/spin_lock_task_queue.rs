use crate::coroutine::coroutine::CoroutineImpl;
use crate::utils::spin_lock_queue::SpinLockQueue;

pub struct SpinLockTaskQueue {
    queue: SpinLockQueue<CoroutineImpl>,
}

impl SpinLockTaskQueue {
    pub fn new() -> Self {
        SpinLockTaskQueue {
            queue: SpinLockQueue::<CoroutineImpl>::new(),
        }
    }

    pub fn push(&self, task: CoroutineImpl) {
        self.queue.push(task);
    }

    pub fn pop(&self) -> Option<CoroutineImpl> {
        self.queue.pop()
    }
}

unsafe impl Send for SpinLockTaskQueue {}
unsafe impl Sync for SpinLockTaskQueue {}