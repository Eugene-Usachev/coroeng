#[cfg(any(target_os = "linux", target_os = "android"))]
#[path = "epoll/mod.rs"]
pub mod epoll;
pub mod io_uring;

#[cfg(any(target_os = "linux", target_os = "android"))]
#[path = "epoll/mod.rs"]
pub use epoll::*;
pub use io_uring::*;