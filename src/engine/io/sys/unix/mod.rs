#[cfg(any(target_os = "linux", target_os = "android"))]
#[path = "epoll/mod.rs"]
pub mod epoll;
mod io_uring;

#[cfg(any(target_os = "linux", target_os = "android"))]
#[path = "epoll/mod.rs"]
pub use epoll::*;