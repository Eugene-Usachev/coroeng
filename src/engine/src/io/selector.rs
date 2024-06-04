//! This module contains a description of [`Selector`].
//!
//! Selector is a trait for working with systems selectors like epoll, kqueue, io_uring etc.

use crate::import_fd_for_os;
import_fd_for_os!();
use crate::io::PollState;
use crate::scheduler::Scheduler;
use crate::utils::Ptr;

/// Selector is a trait for working with systems selectors like epoll, kqueue, io_uring etc.
///
/// The main concept is as follows:
///
/// * Interaction with the selector happens through the required [`PollState`] and by the [`Selector::poll`],
/// which processes coroutines that are ready.
///
/// * Each [`PollState`] contains all the necessary information for making a syscall.
///
/// * Upon completion of the system call, [`Selector`] will wake the coroutine up.
///
/// * If it's necessary to return the result of the syscall, the [`PollState`] contains a pointer to the result variable.
///
/// * The [`Selector`] itself will awaken the coroutine when it's ready.
pub trait Selector {
    /// Returns true, if the [`Selector`] needs to have the [`PollState`] re-registered fd every time.
    /// [`EpolledSelector`](crate::io::sys::unix::EpolledSelector) returns false.
    /// [`IoUringSelector`](crate::io::sys::unix::IoUringSelector) returns true.
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
    ///
    /// # Return
    ///
    /// Returns true, if [`end`](crate::coroutine::YieldStatus::End) was handled.
    fn poll(&mut self, scheduler: &mut Scheduler) -> Result<bool, ()>;
    /// Registers the [`PollState`] with the selector.
    fn register(&mut self, state_ptr: Ptr<PollState>);
    /// Deregisters the [`PollState`] with the selector by the fd.
    ///
    /// # Note
    ///
    /// After deregistering, the [`PollState`] will be ignored in [`Selector::poll`].
    fn deregister(&mut self, fd: RawFd);

    // TODO need test, because I change was_written to Buffer::offset()
    /// Tells the selector that [`WriteTcpState`](crate::io::WriteTcpState) or another writable [`PollState`] is ready.
    /// So, this method returns before the write syscall is done. The writing will be done in [`Selector::poll`].
    ///
    /// # Panics
    ///
    /// Will lead to panic if the [`PollState`] does not exist at the time of [`Selector::poll`].
    ///
    /// # Note
    ///
    /// This method will lead to one syscall. Only the part of the buffer will be written.
    /// The number of bytes written will be stored in the result variable.
    fn write(&mut self, state_ref: Ptr<PollState>);
    /// Tells the selector that [`WriteAllTcpState`](crate::io::WriteAllTcpState) or another writable [`PollState`] is ready.
    /// So, this method returns before the write syscall is done. The writing will be done in [`Selector::poll`].
    ///
    /// # Panics
    ///
    /// Will lead to panic if the [`PollState`] does not exist at the time of [`Selector::poll`].
    ///
    /// # Note
    ///
    /// This method can lead to one or more syscalls.
    fn write_all(&mut self, state_ref: Ptr<PollState>);
    /// TODO docs
    fn close_connection(&mut self, state_ref: Ptr<PollState>);
}