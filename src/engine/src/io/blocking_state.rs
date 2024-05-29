use std::fmt::Debug;
use std::io::Error;
use std::net::SocketAddr;
use crate::coroutine::CoroutineImpl;
use crate::net::TcpStream;

pub struct ConnectTcpState {
    pub(crate) address: SocketAddr,
    pub(crate) coroutine: CoroutineImpl,
    pub(crate) result: *mut Result<TcpStream, Error>
}

pub enum BlockingState {
    ConnectTcp(Box<ConnectTcpState>)
}

impl BlockingState {
    #[inline(always)]
    pub(crate) fn new_connect_tcp(address: SocketAddr, coroutine: CoroutineImpl, result: *mut Result<TcpStream, Error>) -> Self {
        BlockingState::ConnectTcp(Box::new(ConnectTcpState { address, coroutine, result }))
    }
}

impl Debug for BlockingState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BlockingState::ConnectTcp(state) => {
                write!(f, "ConnectTcp to addr {:?}", state.address)
            }
        }
    }
}