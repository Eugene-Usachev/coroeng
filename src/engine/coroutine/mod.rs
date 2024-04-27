//! # [`coroutine`]
//!
//! This module contains a description of [`Coroutine`], [`YieldStatus`], [`CoroutineImpl`] for working with the scheduler.
//! You can use this module for low-level work with the scheduler.
//!
//! # [`yielding`]
//! This module contains functions for the high-level working with the scheduler. For example, [`yield_now`].

pub mod coroutine;
pub mod yielding;

pub use coroutine::*;
pub use yielding::*;
