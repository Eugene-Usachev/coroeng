// TODO docs

use std::io::Error;
use std::fmt::{Debug, Formatter};
import_fd_for_os!();
use socket2::{SockAddr, Socket};
use crate::coroutine::coroutine::CoroutineImpl;
use crate::net::tcp::TcpStream;
use crate::buf::Buffer;
use crate::import_fd_for_os;
use crate::utils::{bits, Ptr};

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

pub struct PollState {
    pub(crate) fd: RawFd,
    pub(crate) coroutine: CoroutineImpl,
    pub(crate) result: *mut Result<Buffer, Error>
}

pub struct RecvState {
    pub(crate) fd: RawFd,
    pub(crate) buffer: Buffer,
    pub(crate) coroutine: CoroutineImpl,
    pub(crate) result: *mut Result<Buffer, Error>
}

pub struct SendState {
    pub(crate) fd: RawFd,
    pub(crate) buffer: Buffer,
    pub(crate) coroutine: CoroutineImpl,
    pub(crate) result: *mut Result<Option<Buffer>, Error>
}

pub struct SendAllState {
    pub(crate) fd: RawFd,
    pub(crate) buffer: Buffer,
    pub(crate) coroutine: CoroutineImpl,
    pub(crate) result: *mut Result<(), Error>
}

pub struct CloseState {
    pub(crate) fd: RawFd,
    pub(crate) coroutine: CoroutineImpl,
    pub(crate) result: *mut Result<(), Error>
}

// TODO Update for not box
/// # Why using [`Box`]?
///
/// Typically, most states are [`PollState`], which weighs 32 bytes (40 including the enum itself).
/// At this time, the heaviest states considering the enum itself weigh 80 bytes, which makes the entire enum [`State`] weigh 80 bytes.
/// To avoid this, states are stacked in a [`Box`], thereby allowing the enum itself to weigh 16 bytes (any state weighs 16 bytes + its own weight, except [`EmptyState`]).
/// This allows to reduce the overall weight of all states (in the example with [`PollState`] State([`PollState`]) now weighs 48 bytes).
/// Since states are only used in IO operations, which are much more expensive than dereferencing, there is no performance impact.
pub enum State {
    Empty(EmptyState),
    AcceptTcp(Ptr<AcceptTcpState>),
    ConnectTcp(Ptr<ConnectTcpState>),
    Poll(Ptr<PollState>),
    Recv(Ptr<RecvState>),
    /// Tells the selector that [`SendState`](SendState) or another writable [`State`] is ready.
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
    Send(Ptr<SendState>),
    /// Tells the selector that [`SendAllState`](crate::io::SendAllState) or another writable [`State`] is ready.
    /// So, this method returns before the write syscall is done. The writing will be done in [`Selector::poll`].
    ///
    /// # Panics
    ///
    /// Will lead to panic if the [`State`] does not exist at the time of [`Selector::poll`].
    ///
    /// # Note
    ///
    /// This method can lead to one or more syscalls.
    SendAll(Ptr<SendAllState>),
    Close(Ptr<CloseState>)
}

pub struct StateManager {
    state_ptr_pool: Vec<Ptr<State>>,
    accept_tcp_pool: Vec<Ptr<AcceptTcpState>>,
    connect_tcp_pool: Vec<Ptr<ConnectTcpState>>,
    poll_tcp_pool: Vec<Ptr<PollState>>,
    read_tcp_pool: Vec<Ptr<RecvState>>,
    write_tcp_pool: Vec<Ptr<SendState>>,
    write_all_tcp_pool: Vec<Ptr<SendAllState>>,
    close_tcp_pool: Vec<Ptr<CloseState>>
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
            State::Poll(state) => self.poll_tcp_pool.push(state),
            State::Recv(state) => self.read_tcp_pool.push(state),
            State::Send(state) => self.write_tcp_pool.push(state),
            State::SendAll(state) => self.write_all_tcp_pool.push(state),
            State::Close(state) => self.close_tcp_pool.push(state)
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
                state.write(PollState { fd, coroutine, result });
                State::Poll(state)
            }
            None => {
                State::Poll(Ptr::new(PollState { fd, coroutine, result }))
            }
        }
    }
    
    #[inline(always)]
    pub fn read_tcp(&mut self, fd: RawFd, buffer: Buffer, coroutine: CoroutineImpl, result: *mut Result<Buffer, Error>) -> State {

        match self.read_tcp_pool.pop() {
            Some(state) => unsafe {
                state.write(RecvState { fd, buffer, coroutine, result });
                State::Recv(state)
            }
            None => {
                State::Recv(Ptr::new(RecvState { fd, buffer, coroutine, result }))
            }
        }
    }
    
    #[inline(always)]
    pub fn write_tcp(&mut self, fd: RawFd, buffer: Buffer, coroutine: CoroutineImpl, result: *mut Result<Option<Buffer>, Error>) -> State {

        match self.write_tcp_pool.pop() {
            Some(state) => unsafe {
                state.write(SendState { fd, buffer, coroutine, result });
                State::Send(state)
            }
            None => {
                State::Send(Ptr::new(SendState { fd, buffer, coroutine, result }))
            }
        }
    }
    
    #[inline(always)]
    pub fn write_all_tcp(&mut self, fd: RawFd, buffer: Buffer, coroutine: CoroutineImpl, result: *mut Result<(), Error>) -> State {

        match self.write_all_tcp_pool.pop() {
            Some(state) => unsafe {
                state.write(SendAllState { fd, buffer, coroutine, result });
                State::SendAll(state)
            }
            None => {
                State::SendAll(Ptr::new(SendAllState { fd, buffer, coroutine, result }))
            }
        }
    }
    
    #[inline(always)]
    pub fn close_tcp(&mut self, fd: RawFd, coroutine: CoroutineImpl, result: *mut Result<(), Error>) -> State {

        match self.close_tcp_pool.pop() {
            Some(state) => unsafe {
                state.write(CloseState { fd, coroutine, result });
                State::Close(state)
            }
            None => {
                State::Close(Ptr::new(CloseState { fd, coroutine, result }))
            }
        }
    }
}

macro_rules! generate_fd_with_flag_and_set_fd {
    ($($state:ident,)*) => {
        /// Get the file descriptor associated with this state and `is_registered` flag. Not all states have a file descriptor.
        ///
        /// All fd in states use first bit for `is_registered` flag. 
        /// Be careful when using it, better call methods [`is_registered`](#method.is_registered) and [`fd`](#method.fd).
        pub fn fd_with_flag(&self) -> RawFd {
            match self {
                State::Empty(state) => { state.fd }
                $(
                    State::$state(state) => unsafe { state.as_ref().fd }
                )*
                _ => { panic!("[BUG] tried to get fd from {self:?} token") }
            }
        }
        
        /// Set the `is_registered` flag to true for this state.
        pub fn set_is_registered_true(&mut self) {
            match self {
                State::Empty(state) => { 
                    state.fd = bits::set_msb(state.fd)
                }
                $(
                    State::$state(state_ptr) => unsafe {
                        let state_ref = state_ptr.as_mut();
                        state_ref.fd = bits::set_msb(state_ref.fd)
                    }
                )*
                _ => { panic!("[BUG] tried to get fd from {self:?} token") }
            }
        }
    };
}

impl State {
    generate_fd_with_flag_and_set_fd! {
        AcceptTcp,
        Poll,
        Recv,
        Send,
        SendAll,
        Close,
    }
    
    /// Get the file descriptor associated with this state.
    #[inline(always)]
    pub fn fd(&self) -> RawFd {
        bits::clear_msb(self.fd_with_flag())
    }
    
    /// Get the `is_registered` flag associated with this state.
    #[inline(always)]
    pub fn is_registered(&self) -> bool {
        bits::get_msb(self.fd_with_flag())
    }

    #[inline(always)]
    pub fn do_empty(&mut self, fd: RawFd, manager: &mut StateManager) {
        unsafe { std::ptr::write(self, manager.empty(fd)) };
    }

    pub fn new_empty(fd: RawFd) -> Self {
        State::Empty(EmptyState { fd })
    }
}

impl Ptr<State> {
    pub fn rewrite_state(&mut self, new_state: State, manager: &mut StateManager) {
        unsafe {
            manager.put_state(self.replace(new_state)); 
        }
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
            State::Poll(state) => unsafe { write!(f, "Poll, fd: {:?}", state.as_ref().fd) }
            State::Recv(state) => unsafe { write!(f, "Recv, fd: {:?}", state.as_ref().fd) }
            State::Send(state) => unsafe { write!(f, "Send, fd: {:?}", state.as_ref().fd) }
            State::SendAll(state) => unsafe { write!(f, "SendAll, fd: {:?}", state.as_ref().fd) }
            State::Close(state) => unsafe { write!(f, "Close, fd: {:?}", state.as_ref().fd) }
        }
    }
}