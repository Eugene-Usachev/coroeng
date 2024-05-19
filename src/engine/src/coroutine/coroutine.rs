//! This module contains a description of [`Coroutine`], [`CoroutineImpl`] for working with the scheduler.
//! This module is used for low-level work with the scheduler.
use std::pin::Pin;
use std::ops::{Coroutine as StdCoroutine};
use crate::coroutine::YieldStatus;

/// The alias for [`StdCoroutine`]<Yield=[`YieldStatus`], Return=()>.
/// The scheduler works only with this type of the coroutines.
pub trait Coroutine = StdCoroutine<Yield=YieldStatus, Return=()>;

/// The alias for [`Pin`]<[`Box`]<dyn [`Coroutine`]>>.
/// The scheduler works only with this type of the coroutines.
pub type CoroutineImpl = Pin<Box<dyn Coroutine>>;