use std::time::{Duration, Instant};
use crate::coroutine::coroutine::{CoroutineImpl};
use crate::coroutine::YieldStatus;

pub(crate) struct SleepingCoroutine {
    pub(crate) execution_time: Instant,
    pub(crate) co: CoroutineImpl,
}

impl SleepingCoroutine {
    pub fn new(dur: Duration, co: CoroutineImpl) -> Self {
        Self {
            execution_time: Instant::now() + dur,
            co
        }
    }
}

pub fn sleep(dur: Duration) -> YieldStatus {
    YieldStatus::sleep(dur)
}

unsafe impl Send for SleepingCoroutine {}
