#[macro_export]
macro_rules! import_fd_for_os {
    () => {
        #[cfg(unix)]
        use std::os::fd::{RawFd as RawFd};
        #[cfg(not(unix))]
        use std::os::windows::io::RawHandle as RawFd;
    };
}