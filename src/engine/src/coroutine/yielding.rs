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

/// Returns [`YieldStatus::End`]. If yielded, the [`scheduler`](crate::scheduler::Scheduler) will be terminated.
///
/// # Be careful
///
/// It means, that [`uninit`](crate::run::uninit) will be called.
/// After this [`Selector`](crate::io::selector::Selector) will be dropped and all poll states will be leaked (with memory).
///
/// # Do not call this function in a production!
/// Because it can lead to a memory leak and coroutine leak (that can cause a deadlock). It uses only for test and recommended to use it only for testing.
pub fn end(_res: *mut ()) -> YieldStatus {
    YieldStatus::end()
}