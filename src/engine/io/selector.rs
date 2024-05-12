//! This module contains a description of [`Selector`].
//!
//! Selector is a trait for working with systems selectors like epoll, kqueue, io_uring etc.

use std::os::fd::RawFd;
// TODO: here in docs we use "the selector" instead of [`Selector`] and "register with the selector" instead of something better.
use crate::engine::io::sys::unix::{EpolledSelector, IoUringSelector};
use crate::engine::io::State;
use crate::engine::local::Scheduler;
// needed for docs.
use crate::engine::io::state::{WriteTcpState, WriteAllTcpState};
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
pub(crate) trait Selector {
    /// Returns true, if the [`Selector`] needs to have the [`State`] re-registered fd every time.
    /// [`EpolledSelector`] returns false.
    /// [`IoUringSelector`] returns true.
    fn need_reregister(&self) -> bool;
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
    /// Deregisters the [`State`] with the selector by the fd.
    ///
    /// # Note
    ///
    /// After deregistering, the [`State`] will be ignored in [`Selector::poll`].
    fn deregister(&mut self, fd: RawFd);

    /// Tells the selector that [`WriteTcpState`] or another writable [`State`] is ready.
    /// So, this method returns before the write syscall is done. The writing will be done in [`Selector::poll`].
    ///
    /// # Panics
    ///
    /// Will lead to panic if the [`State`] does not exist at the time of [`Selector::poll`].
    ///
    /// # Note
    ///
    /// This method will lead to one syscall. Only the part of the buffer will be written.
    /// The number of bytes written will be stored in the result variable.
    fn write(&mut self, state_ref: Ptr<State>);
    /// Tells the selector that [`WriteAllTcpState`] or another writable [`State`] is ready.
    /// So, this method returns before the write syscall is done. The writing will be done in [`Selector::poll`].
    ///
    /// # Panics
    ///
    /// Will lead to panic if the [`State`] does not exist at the time of [`Selector::poll`].
    ///
    /// # Note
    ///
    /// This method can lead to one or more syscalls.
    fn write_all(&mut self, state_ref: Ptr<State>);
    /// TODO docs
    fn close_connection(&mut self, state_ref: Ptr<State>);
}