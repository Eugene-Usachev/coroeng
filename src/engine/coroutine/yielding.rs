//! This module contains functions for the high-level working with the scheduler. For example, [`yield_now`].
use crate::engine::coroutine::coroutine::YieldStatus;

/// Returns [`YieldStatus::Yield`]. If yielded, the scheduler will wake the coroutine up later.
///
/// # Example
///
/// ```rust
/// // work here
/// yield $crate::engine::coroutine::yield_now(); // let the scheduler wake other coroutines up.
/// // work here after some time
/// ```
pub fn yield_now() -> YieldStatus {
    YieldStatus::Yield
}

/// Returns [`YieldStatus::Never`].
///
/// # Panics
///
/// will panic if yielded.
///
/// # Example
///
/// ```rust
/// macro_rules! new_coroutine {
///     ($code:block) => {
///         || {
///             if true {
///                 $code
///             } else {
///                 yield $crate::engine::coroutine::never();
///             }
///         }
///     }
/// }
/// ```
pub fn never() -> YieldStatus {
    YieldStatus::Never
}