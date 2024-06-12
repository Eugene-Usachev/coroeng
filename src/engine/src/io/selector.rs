//! This module contains a description of [`Selector`].
//!
//! Selector is a trait for working with systems selectors like epoll, kqueue, io_uring etc.

// TODO update docs, because it have no epoll, write, write_all, close_connection, need_reregister, deregister now
use crate::io::State;
use crate::scheduler::Scheduler;
use crate::utils::Ptr;

/// Selector is a trait for working with systems selectors like epoll, kqueue, io_uring etc.
///
/// The main concept is as follows:
///
/// * Interaction with the selector happens through the required [`State`] and by the [`Selector::poll`],
/// which processes coroutines that are ready.
///
/// * Each [`State`] contains all the necessary information for making a syscall.
///
/// * Upon completion of the system call, [`Selector`] will wake the coroutine up.
///
/// * If it's necessary to return the result of the syscall, the [`State`] contains a pointer to the result variable.
///
/// * The [`Selector`] itself will awaken the coroutine when it's ready.
pub trait Selector {
    /// Polls the [`Selector`] for coroutines that are ready.
    /// This method will wake the coroutines up.
    ///
    /// # Note
    ///
    /// This method is long-running.
    /// It will poll the [`Selector`] (this step can take a millisecond) and wake the coroutines up.
    ///
    /// So, you should call this method, when no ready coroutines.
    fn poll(&mut self, scheduler: &mut Scheduler) -> Result<(), ()>;
    /// Registers the [`State`] with the selector.
    fn register(&mut self, state_ptr: Ptr<State>);
}