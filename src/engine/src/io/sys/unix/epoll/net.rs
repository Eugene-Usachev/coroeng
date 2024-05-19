//! This module contains functions for working with the network with the epoll.

use std::net::{IpAddr, SocketAddr};
use std::os::fd::{AsRawFd, BorrowedFd, OwnedFd};
use libc::{linger, O_NONBLOCK, SYS_fcntl, F_SETFL, syscall};
use nix::sys::socket::{AddressFamily, Backlog, listen, setsockopt, SockType, SockFlag, SockProtocol, bind, SockaddrIn};
use nix::sys::socket::sockopt::{Linger, ReuseAddr, ReusePort, TcpNoDelay};
use crate::io::sys::unix::epoll::check_error::check_error;

/// The value of `SO_REUSEADDR`, `TcpNoDelay` and `SO_REUSEPORT`
const OPTVAL: bool = true;

/// Returns [`OwnedFd`] for the configured tcp listener.
#[inline]
pub(crate) fn get_tcp_listener_fd(socket_addr: SocketAddr) -> OwnedFd {
    // TODO v6
    let octets;
    match socket_addr.ip() {
        IpAddr::V4(ip) => {
            octets = ip.octets();
        }
        IpAddr::V6(_) => {panic!("IPv6 is not supported")}
    }
    let fd = nix::sys::socket::socket(
        AddressFamily::Inet,
        SockType::Stream,
        SockFlag::SOCK_NONBLOCK,
        SockProtocol::Tcp
    ).expect("cannot create socket");

    setsockopt(&fd, ReuseAddr, &OPTVAL).expect("cannot set SO_REUSEADDR");
    setsockopt(&fd, ReusePort, &OPTVAL).expect("cannot set SO_REUSEPORT");

    bind(fd.as_raw_fd(), &SockaddrIn::new(octets[0], octets[1], octets[2], octets[3], socket_addr.port())).expect("cannot bind");
    listen(&fd, Backlog::new(1024).unwrap()).expect("cannot listen");

    fd
}

/// Sets a connection up for non-blocking IO.
///
/// # Panics
///
/// If SETSOCKOPT fails or fcntl fails. This is impossible if provided with a valid socket fd.
#[inline]
pub(crate) fn setup_connection(fd: &BorrowedFd) {
    unsafe {
        setsockopt(fd, TcpNoDelay, &OPTVAL).expect("cannot set TCP_NODELAY");
        // This code decrease performance by 50%. I don't know why.
        //
        // check_error(
        //     syscall(SYS_setsockopt, fd.as_raw_fd(), SOL_SOCKET, SO_INCOMING_CPU, &core_id.id, core::mem::size_of_val(&core_id.id)),
        //     "cannot set SO_INCOMING_CPU", true
        // );
        set_nonblocking(fd);
    }
}

/// Sets non-blocking IO for a file descriptor.
///
/// # Panics
///
/// If the syscall fails. In theory this is impossible.
#[inline]
pub(crate) unsafe fn set_nonblocking(fd: &BorrowedFd) {
    unsafe {
        check_error(syscall(SYS_fcntl, fd.as_raw_fd(), F_SETFL, O_NONBLOCK), "cannot set nonblocking", true);
    }
}

/// Closes a connection.
///
/// # Panics
///
/// If the syscall fails. For example, if the connection is already closed.
#[inline(always)]
pub(crate) unsafe fn close_connection(conn_fd: &BorrowedFd) {
    const OPTVAL_SOLINGER_TIMEOUT: linger = linger { l_onoff: 1, l_linger: 0 };
    setsockopt(conn_fd, Linger, &OPTVAL_SOLINGER_TIMEOUT).expect("");
    nix::unistd::close(conn_fd.as_raw_fd()).expect("Failed to close conn_fd");
}