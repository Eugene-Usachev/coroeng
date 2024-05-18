//! # [`coroutine`]
//!
//! This module contains a description of [`Coroutine`], [`CoroutineImpl`] for working with the scheduler.
//! This module is used for low-level work with the scheduler.
//!
//! # [`yielding`]
//! This module contains functions for the high-level working with the scheduler. For example, [`yield_now`].
//!
//! # [`yield_status`]
//! This module contains a description of [`YieldStatus`] for low-level work with the scheduler.
//! Please use high-level functions for working with the scheduler if it is possible.

pub mod coroutine;
pub mod yielding;
pub mod yield_status;

pub use coroutine::*;
pub use yielding::*;
pub use yield_status::*;
