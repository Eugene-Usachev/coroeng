mod controller;

use std::cmp::Ordering;
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

pub fn sleep(dur: Duration, _res: *mut ()) -> YieldStatus {
    YieldStatus::sleep(dur)
}

unsafe impl Send for SleepingCoroutine {}

impl Eq for SleepingCoroutine {}

impl PartialEq<Self> for SleepingCoroutine {
    fn eq(&self, other: &Self) -> bool {
        self.execution_time == other.execution_time
    }
}

impl PartialOrd<Self> for SleepingCoroutine {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.execution_time.partial_cmp(&other.execution_time)
    }
}

impl Ord for SleepingCoroutine {
    fn cmp(&self, other: &Self) -> Ordering {
        self.execution_time.cmp(&other.execution_time)
    }
}
