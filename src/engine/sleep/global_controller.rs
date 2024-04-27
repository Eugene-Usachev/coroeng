use std::intrinsics::unlikely;
use std::mem;
use std::time::Instant;
use std::sync::Mutex;
use crate::engine::work_stealing::scheduler::SCHEDULER;
use crate::engine::work_stealing::sleep::sleep::SleepingCoroutine;

pub(crate) struct GlobalController {
    list: Mutex<Vec<SleepingCoroutine>>
}

impl GlobalController {
    pub(crate) fn new() -> Self {
        Self {
            list: Mutex::new(Vec::with_capacity(8)),
        }
    }

    pub(crate) fn add(&self, co: SleepingCoroutine) {
        self.list.lock().unwrap().push(co);
    }

    pub(crate) fn awake_coroutines(&self) {
        let mut list = self.list.lock().unwrap();
        let mut new_list = Vec::with_capacity(list.len());
        mem::swap(&mut new_list, &mut list);
        let now = Instant::now();
        for sleeping_coroutine in new_list.into_iter() {
            if unlikely(now >= sleeping_coroutine.execution_time) {
                SCHEDULER.sched(sleeping_coroutine.co);
                continue;
            }
            list.push(sleeping_coroutine)
        }
    }
}