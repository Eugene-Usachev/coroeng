use std::io;
use nix::libc::c_long;

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