//! This module is for unix epoll. It provides [`EpolledSelector`] for working with the epoll.

pub mod net;
pub mod check_error;
pub mod selector;

pub use selector::*;