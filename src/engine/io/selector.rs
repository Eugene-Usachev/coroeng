//! This module contains a description of [`Selector`].
//!
//! Selector is a trait for working with systems selectors like epoll, kqueue, io_uring etc.

use std::os::fd::RawFd;
use crate::engine::io::Token;
use crate::engine::local::Scheduler;
// needed for docs.
use crate::engine::io::token::{EmptyToken, WriteTcpToken, WriteAllTcpToken};

/// Selector is a trait for working with systems selectors like epoll, kqueue, io_uring etc.
///
/// The main concept is as follows:
///
/// * Interaction with the selector happens through the required [`Token`] and by the [`Selector::poll`],
/// which processes coroutines that are ready.
///
/// * Each token contains all the necessary information for making a syscall.
///
/// * Upon completion of the system call, [`Selector`] will wake the coroutine up.
///
/// * If it's necessary to return the result of the syscall, the token contains a pointer to the result variable.
///
/// * The token will be replaced with [`EmptyToken`] by one after the syscall completes.
///
/// * The selector itself will awaken the coroutine when it's ready.
///
/// * To invoke asynchronous functions, the identifier of the required token is used.
///
/// * To get the token identifier, uses [`Selector::insert_token`].
///
/// * For replacing the token, uses [`Selector::get_token_mut_ref`].
pub(crate) trait Selector {
    /// Insert token into the selector, returns the token identifier.
    fn insert_token(&mut self, token: Token) -> usize;
    /// Returns mut reference to the token by identifier. Uses for replacing the token.
    ///
    /// # Panics
    /// If the token doesn't exist.
    ///
    /// # Example
    ///
    /// ```rust
    /// let token = selector.get_token_mut_ref(stream);
    /// *token = Token::new_write_tcp(token.fd(), buf, task, result);
    /// ```
    fn get_token_mut_ref(&mut self, token_id: usize) -> &mut Token;
    /// Polls the selector for coroutines that are ready.
    /// This method will wake the coroutines up.
    ///
    /// # Note
    ///
    /// This method is long-running.
    /// It will poll the selector (this step can take a millisecond) and wake the coroutines up.
    ///
    /// So, you should call this method, when no ready coroutines.
    fn poll(&mut self, scheduler: &mut Scheduler) -> Result<(), ()>;
    /// Registers the token with the selector.
    fn register(&mut self, fd: RawFd, token_id: usize);
    /// Deregisters the token with the selector by the token identifier.
    ///
    /// # Panics
    ///
    /// If the token doesn't exist.
    ///
    /// # Note
    ///
    /// After deregistering, the token will be ignored in [`Selector::poll`].
    ///
    /// # Be careful
    ///
    /// If the token id is still in use, for example after call [`Selector::write`] but before [`Selector::poll`],
    /// it will lead to panic.
    fn deregister(&mut self, token_id: usize) -> Token;

    /// Tells the selector that [`WriteTcpToken`] or another writable token is ready.
    /// So, this method returns before the write syscall is done. The writing will be done in [`Selector::poll`].
    ///
    /// # Panics
    ///
    /// Will lead to panic if the token does not exist at the time of [`Selector::poll`].
    ///
    /// # Note
    ///
    /// This method will lead to one syscall. Only the part of the buffer will be written.
    /// The number of bytes written will be stored in the result variable.
    fn write(&mut self, token_id: usize);
    /// Tells the selector that [`WriteAllTcpToken`] or another writable token is ready.
    /// So, this method returns before the write syscall is done. The writing will be done in [`Selector::poll`].
    ///
    /// # Panics
    ///
    /// Will lead to panic if the token does not exist at the time of [`Selector::poll`].
    ///
    /// # Note
    ///
    /// This method can lead to one or more syscalls.
    fn write_all(&mut self, token_id: usize);
    /// TODO: docs
    fn close_connection(token: Token);
}