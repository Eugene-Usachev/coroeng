//! This module contains functions for working with the network with the epoll.

use std::net::{IpAddr, SocketAddr};
use std::os::fd::{AsRawFd, OwnedFd};
use nix::sys::socket::{AddressFamily, Backlog, listen, setsockopt, SockType, SockFlag, SockProtocol, bind, SockaddrIn};
use nix::sys::socket::sockopt::{ReuseAddr, ReusePort};

/// The value of `SO_REUSEADDR`, `TcpNoDelay` and `SO_REUSEPORT`
const OPTVAL: bool = true;

// TODO result
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