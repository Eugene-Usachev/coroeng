//! This module provides a function, that checks the result of a syscall.
//!
//! Read [`check_error`] for more info.

use std::io;
use nix::libc::c_long;

/// Checks the result of a syscall.
///
/// If the result is a error, it prints or panics (depending on `is_fatal` argument) an error message.
///
/// # Arguments
///
/// * `res` - The result of the syscall.
/// * `msg` - The message to print or panic if the result is an error.
/// * `is_fatal` - Whether the error should be printed or not.
///
/// # Panics
///
/// If the result is an error and `is_fatal` is `true`.
///
/// # Examples
///
/// ```Rust
/// // panics with message "[FATAL] cannot set nonblocking: {reason}" if syscall fails.
/// check_error(syscall(SYS_fcntl, fd.as_raw_fd(), F_SETFL, O_NONBLOCK), "cannot set nonblocking", true);
/// ```
#[inline(always)]
pub fn check_error(res: c_long, msg: &str, is_fatal: bool) {
    if res < 0 {
        if is_fatal {
            panic!("[FATAL] {}: {}", msg, io::Error::last_os_error());
        } else {
            eprintln!("[ERROR] {}: {}", msg, io::Error::last_os_error());
        }
    }
}