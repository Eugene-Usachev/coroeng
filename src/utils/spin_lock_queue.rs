use std::hint::spin_loop;
use std::intrinsics::unlikely;
use crossbeam::queue::ArrayQueue;

const QUEUE_SIZE: usize = 1024;

pub struct SpinLockQueue<T> {
    queue: ArrayQueue<T>,
}

impl<T> SpinLockQueue<T> {
    pub fn new() -> Self {
        SpinLockQueue {
            queue: ArrayQueue::new(QUEUE_SIZE)
        }
    }

    pub fn push(&self, task: T) {
        let mut old = self.queue.force_push(task);

        if unlikely(old.is_some()) {
            loop {
                for _ in 0..10 {
                    spin_loop();
                }

                unsafe {
                    old = self.queue.force_push(old.unwrap_unchecked());
                    if old.is_none() {
                        break;
                    }
                }
            }
        }
    }

    pub fn pop(&self) -> Option<T> {
        self.queue.pop()
    }
}