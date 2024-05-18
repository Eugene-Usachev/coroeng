//! This module is for unix epoll. It provides [`EpolledSelector`] for working with the epoll.

pub(crate) mod net;
pub(crate) mod check_error;
pub(crate) mod selector;

pub(crate) use selector::*;