//! This module contains functions for the high-level working with the scheduler. For example, [`yield_now`].
use crate::coroutine::YieldStatus;

/// Returns [`YieldStatus::Yield`]. If yielded, the [`scheduler`](crate::scheduler::Scheduler) will wake the coroutine up later.
///
/// # Example
///
/// ```ignore
/// use engine::coroutine::yield_now;
/// use proc::coro;
///
/// #[coro]
/// fn func_with_yield() {
///     // work here
///     yield yield_now(); // let the scheduler wake other coroutines up.
///     // work here after some time
/// }
/// ```
pub fn yield_now(_res: *mut ()) -> YieldStatus {
    YieldStatus::yield_now()
}