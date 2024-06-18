// TODO docs

use std::io::Error;
use std::fmt::{Debug, Formatter};
import_fd_for_os!();
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use crate::coroutine::coroutine::CoroutineImpl;
use crate::net::tcp::TcpStream;
use crate::buf::Buffer;
use crate::import_fd_for_os;
use crate::utils::Ptr;

pub struct EmptyState {
    fd: RawFd
}

pub struct AcceptTcpState {
    pub(crate) fd: RawFd,
    pub(crate) coroutine: CoroutineImpl,
    pub(crate) result: *mut Result<TcpStream, Error>,
    
}

pub struct ConnectTcpState {
    pub(crate) address: SockAddr,
    pub(crate) socket: Socket,
    pub(crate) coroutine: CoroutineImpl,
    pub(crate) result: *mut Result<TcpStream, Error>
}

pub struct PollTcpState {
    pub(crate) fd: RawFd,
    pub(crate) coroutine: CoroutineImpl,
    pub(crate) result: *mut Result<Buffer, Error>
}

pub struct ReadTcpState {
    pub(crate) fd: RawFd,
    pub(crate) buffer: Buffer,
    pub(crate) coroutine: CoroutineImpl,
    pub(crate) result: *mut Result<Buffer, Error>
}

pub struct WriteTcpState {
    pub(crate) fd: RawFd,
    pub(crate) buffer: Buffer,
    pub(crate) coroutine: CoroutineImpl,
    pub(crate) result: *mut Result<Option<Buffer>, Error>
}

pub struct WriteAllTcpState {
    pub(crate) fd: RawFd,
    pub(crate) buffer: Buffer,
    pub(crate) coroutine: CoroutineImpl,
    pub(crate) result: *mut Result<(), Error>
}

pub struct CloseTcpState {
    pub(crate) fd: RawFd,
    pub(crate) coroutine: CoroutineImpl,
    pub(crate) result: *mut Result<(), Error>
}

// TODO Update for not box
/// # Why using [`Box`]?
///
/// Typically, most states are [`PollTcpState`], which weighs 32 bytes (40 including the enum itself).
/// At this time, the heaviest states considering the enum itself weigh 80 bytes, which makes the entire enum [`State`] weigh 80 bytes.
/// To avoid this, states are stacked in a [`Box`], thereby allowing the enum itself to weigh 16 bytes (any state weighs 16 bytes + its own weight, except [`EmptyState`]).
/// This allows to reduce the overall weight of all states (in the example with [`PollTcpState`] State([`PollTcpState`]) now weighs 48 bytes).
/// Since states are only used in IO operations, which are much more expensive than dereferencing, there is no performance impact.
pub enum State {
    Empty(EmptyState),
    AcceptTcp(Ptr<AcceptTcpState>),
    ConnectTcp(Ptr<ConnectTcpState>),
    PollTcp(Ptr<PollTcpState>),
    ReadTcp(Ptr<ReadTcpState>),
    /// Tells the selector that [`WriteTcpState`](WriteTcpState) or another writable [`State`] is ready.
    /// So, this method returns before the write syscall is done. The writing will be done in [`Selector::poll`].
    ///
    /// # Panics
    ///
    /// Will lead to panic if the [`State`] does not exist at the time of [`Selector::poll`].
    ///
    /// # Note
    ///
    /// This method will lead to one syscall. Only the part of the buffer will be written.
    /// The number of bytes written will be stored in the result variable.
    WriteTcp(Ptr<WriteTcpState>),
    /// Tells the selector that [`WriteAllTcpState`](crate::io::WriteAllTcpState) or another writable [`State`] is ready.
    /// So, this method returns before the write syscall is done. The writing will be done in [`Selector::poll`].
    ///
    /// # Panics
    ///
    /// Will lead to panic if the [`State`] does not exist at the time of [`Selector::poll`].
    ///
    /// # Note
    ///
    /// This method can lead to one or more syscalls.
    WriteAllTcp(Ptr<WriteAllTcpState>),
    CloseTcp(Ptr<CloseTcpState>)
}

pub struct StateManager {
    state_ptr_pool: Vec<Ptr<State>>,
    accept_tcp_pool: Vec<Ptr<AcceptTcpState>>,
    connect_tcp_pool: Vec<Ptr<ConnectTcpState>>,
    poll_tcp_pool: Vec<Ptr<PollTcpState>>,
    read_tcp_pool: Vec<Ptr<ReadTcpState>>,
    write_tcp_pool: Vec<Ptr<WriteTcpState>>,
    write_all_tcp_pool: Vec<Ptr<WriteAllTcpState>>,
    close_tcp_pool: Vec<Ptr<CloseTcpState>>
}

impl StateManager {
    pub fn new() -> Self {
        Self {
            state_ptr_pool: Vec::new(),
            accept_tcp_pool: Vec::new(),
            connect_tcp_pool: Vec::new(),
            poll_tcp_pool: Vec::new(),
            read_tcp_pool: Vec::new(),
            write_tcp_pool: Vec::new(),
            write_all_tcp_pool: Vec::new(),
            close_tcp_pool: Vec::new()
        }
    }
    
    #[inline(always)]
    pub fn get_state_ptr(&mut self) -> Ptr<State> {
        if let Some(state) = self.state_ptr_pool.pop() {
            state
        } else {
            Ptr::new(State::Empty(EmptyState { fd: 0 }))
        }
    }
    
    #[inline(always)]
    pub fn put_state_ptr(&mut self, state: Ptr<State>) {
        self.state_ptr_pool.push(state)
    }
    
    pub fn put_state(&mut self, state: State) {
        match state {
            State::Empty(_) => {}
            State::AcceptTcp(state) => self.accept_tcp_pool.push(state),
            State::ConnectTcp(state) => self.connect_tcp_pool.push(state),
            State::PollTcp(state) => self.poll_tcp_pool.push(state),
            State::ReadTcp(state) => self.read_tcp_pool.push(state),
            State::WriteTcp(state) => self.write_tcp_pool.push(state),
            State::WriteAllTcp(state) => self.write_all_tcp_pool.push(state),
            State::CloseTcp(state) => self.close_tcp_pool.push(state)
        }
    }
    
    #[inline(always)]
    pub fn empty(&mut self, fd: RawFd) -> State {
        State::Empty(EmptyState { fd })
    }
    
    #[inline(always)]
    pub fn accept_tcp(&mut self, fd: RawFd, coroutine: CoroutineImpl, result: *mut Result<TcpStream, Error>) -> State {
        match self.accept_tcp_pool.pop() {
            Some(state) => unsafe {
                state.write(AcceptTcpState { fd, coroutine, result });
                State::AcceptTcp(state)
            }
            None => {
                State::AcceptTcp(Ptr::new(AcceptTcpState { fd, coroutine, result }))
            }
        }
    }
    
    #[inline(always)]
    pub fn connect_tcp(&mut self, address: SockAddr, socket: Socket, coroutine: CoroutineImpl, result: *mut Result<TcpStream, Error>) -> State {

        match self.connect_tcp_pool.pop() {
            Some(state) => unsafe {
                state.write(ConnectTcpState { address, socket, coroutine, result });
                State::ConnectTcp(state)
            }
            None => {
                State::ConnectTcp(Ptr::new(ConnectTcpState { address, socket, coroutine, result }))
            }
        }
    }
    
    #[inline(always)]
    pub fn poll_tcp(&mut self, fd: RawFd, coroutine: CoroutineImpl, result: *mut Result<Buffer, Error>) -> State {

        match self.poll_tcp_pool.pop() {
            Some(state) => unsafe {
                state.write(PollTcpState { fd, coroutine, result });
                State::PollTcp(state)
            }
            None => {
                State::PollTcp(Ptr::new(PollTcpState { fd, coroutine, result }))
            }
        }
    }
    
    #[inline(always)]
    pub fn read_tcp(&mut self, fd: RawFd, buffer: Buffer, coroutine: CoroutineImpl, result: *mut Result<Buffer, Error>) -> State {

        match self.read_tcp_pool.pop() {
            Some(state) => unsafe {
                state.write(ReadTcpState { fd, buffer, coroutine, result });
                State::ReadTcp(state)
            }
            None => {
                State::ReadTcp(Ptr::new(ReadTcpState { fd, buffer, coroutine, result }))
            }
        }
    }
    
    #[inline(always)]
    pub fn write_tcp(&mut self, fd: RawFd, buffer: Buffer, coroutine: CoroutineImpl, result: *mut Result<Option<Buffer>, Error>) -> State {

        match self.write_tcp_pool.pop() {
            Some(state) => unsafe {
                state.write(WriteTcpState { fd, buffer, coroutine, result });
                State::WriteTcp(state)
            }
            None => {
                State::WriteTcp(Ptr::new(WriteTcpState { fd, buffer, coroutine, result }))
            }
        }
    }
    
    #[inline(always)]
    pub fn write_all_tcp(&mut self, fd: RawFd, buffer: Buffer, coroutine: CoroutineImpl, result: *mut Result<(), Error>) -> State {

        match self.write_all_tcp_pool.pop() {
            Some(state) => unsafe {
                state.write(WriteAllTcpState { fd, buffer, coroutine, result });
                State::WriteAllTcp(state)
            }
            None => {
                State::WriteAllTcp(Ptr::new(WriteAllTcpState { fd, buffer, coroutine, result }))
            }
        }
    }
    
    #[inline(always)]
    pub fn close_tcp(&mut self, fd: RawFd, coroutine: CoroutineImpl, result: *mut Result<(), Error>) -> State {

        match self.close_tcp_pool.pop() {
            Some(state) => unsafe {
                state.write(CloseTcpState { fd, coroutine, result });
                State::CloseTcp(state)
            }
            None => {
                State::CloseTcp(Ptr::new(CloseTcpState { fd, coroutine, result }))
            }
        }
    }
}

impl State {
    #[inline(always)]
    pub fn fd(&self) -> RawFd {
        match self {
            State::Empty(state) => { state.fd }
            State::AcceptTcp(state) => unsafe { state.as_ref().fd }
            State::PollTcp(state) => unsafe { state.as_ref().fd }
            State::ReadTcp(state) => unsafe { state.as_ref().fd }
            State::WriteTcp(state) => unsafe { state.as_ref().fd }
            State::WriteAllTcp(state) => unsafe { state.as_ref().fd }
            State::CloseTcp(state) => unsafe { state.as_ref().fd }

            _ => { panic!("[BUG] tried to get fd from {self:?} token") }
        }
    }

    #[inline(always)]
    pub fn do_empty(&mut self, fd: RawFd, manager: &mut StateManager) {
        manager.put_state(unsafe { (self as *mut State).replace(State::new_empty(fd)) });
    }

    pub fn new_empty(fd: RawFd) -> Self {
        State::Empty(EmptyState { fd })
    }
}

impl Debug for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            State::Empty(state) => { write!(f, "Empty, fd: {:?}", state.fd) }
            State::AcceptTcp(state) => unsafe { write!(f, "AcceptTcp, fd: {:?}", state.as_ref().fd) }
            State::ConnectTcp(state) => unsafe {
                write!(
                    f,
                    "ConnectTcp, addr: {:?}",
                    state.as_ref().address
                )
            }
            State::PollTcp(state) => unsafe { write!(f, "PollTcp, fd: {:?}", state.as_ref().fd) }
            State::ReadTcp(state) => unsafe { write!(f, "ReadTcp, fd: {:?}", state.as_ref().fd) }
            State::WriteTcp(state) => unsafe { write!(f, "WriteTcp, fd: {:?}", state.as_ref().fd) }
            State::WriteAllTcp(state) => unsafe { write!(f, "WriteAllTcp, fd: {:?}", state.as_ref().fd) }
            State::CloseTcp(state) => unsafe { write!(f, "CloseTcp, fd: {:?}", state.as_ref().fd) }
        }
    }
}