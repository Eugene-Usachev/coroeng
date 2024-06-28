// TODO new docs top Recv and Send and Close
//! This module contains a description of [`YieldStatus`] for low-level work with the scheduler.
//! Please use high-level functions for working with the scheduler if it is possible.
use std::fmt::{Debug, Formatter};
use std::io::Error;
use std::net::SocketAddr;
use std::path::Path;
use std::time::Duration;
use crate::io::State;
use crate::net::{TcpListener, TcpStream};
use crate::buf::{Buffer};
use crate::fs::file::File;
use crate::fs::OpenOptions;
use crate::utils::Ptr;

macro_rules! impl_debug_for_as_ref_path {
    ($t:ty, $name:expr) => {
        impl Debug for $t {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                let path = self.path.as_ref().as_ref();
                f.debug_struct($name)
                    .field("path", &path)
                    .finish()
            }
        }
    };
}

/// Contains all necessary information for create a new TCP listener.
#[derive(Debug)]
pub struct NewTcpListener {
    /// The address on which [`TcpListener`] will listen.
    pub(crate) address: SocketAddr,
    /// Pointer to store the newly created [`TcpListener`].
    pub(crate) listener_ptr: *mut TcpListener,
}

/// Contains all necessary information for a TCP connect operation.
#[derive(Debug)]
pub struct TcpConnect {
    /// The address on which the [`TcpStream`] will be connected.
    pub(crate) address: SocketAddr,
    /// Pointer to store the newly created [`TcpStream`].
    pub(crate) stream_ptr: *mut Result<TcpStream, Error>,
}

/// Contains all necessary information for a TCP accept operation.
#[derive(Debug)]
pub struct TcpAccept {
    /// The state pointer associated with [`TcpListener`].
    pub(crate) state_ptr: Ptr<State>,
    /// Pointer to store the result of the TCP accept operation.
    /// If success, the result will contain a [`TcpStream`].
    pub(crate) result_ptr: *mut Result<TcpStream, Error>,
}

/// Contains all necessary information for a receive operation.
#[derive(Debug)]
pub struct Recv {
    /// The state pointer associated with a connection.
    pub(crate) state_ptr: Ptr<State>,
    /// Pointer to store the result of the read operation.
    /// If success, the result will contain [`Buffer`].
    pub(crate) result_ptr: *mut Result<Buffer, Error>,
}

/// Contains all necessary information for a send operation.
#[derive(Debug)]
pub struct Send {
    /// The state pointer associated with a connection.
    pub(crate) state_ptr: Ptr<State>,
    /// The buffer containing data to be written.
    pub(crate) buffer: Buffer,
    /// Pointer to store the result of the write operation.
    /// If success, the result will contain [`Buffer`] with the current `offset`
    /// (read [`Buffer`] or [`Buffer::offset`] description for more information).
    pub(crate) result_ptr: *mut Result<Buffer, Error>,
}

/// Contains all necessary information for a send all operation.
#[derive(Debug)]
pub struct SendAll {
    /// The state pointer associated with a connection.
    pub(crate) state_ptr: Ptr<State>,
    /// The buffer containing data to be written.
    pub(crate) buffer: Buffer,
    /// Pointer to store the result of the write operation.
    /// If success, the result will contain [`Buffer`] with the current `offset`
    /// (read [`Buffer`] or [`Buffer::offset`] description for more information).
    pub(crate) result_ptr: *mut Result<Buffer, Error>,
}

/// Contains all necessary information for an open file operation.
pub struct OpenFile {
    /// Path to the file.
    pub(crate) path: Box<dyn AsRef<Path>>,
    /// Options for opening the file.
    pub(crate) options: OpenOptions,
    /// Pointer to store the result of the open file operation.
    /// If success, the result will contain [`File`].
    pub(crate) file_ptr: *mut Result<File, Error>,
}

impl_debug_for_as_ref_path!(OpenFile, "OpenFile");

/// Contains all necessary information for a read operation without an offset.
#[derive(Debug)]
pub struct Read {
    /// The state pointer associated with a reader (like stream, file, etc.).
    pub(crate) state_ptr: Ptr<State>,
    /// Pointer to store the result of the read operation.
    /// If success, the result will contain [`Buffer`].
    pub(crate) result_ptr: *mut Result<Buffer, Error>,
}

/// Contains all necessary information for a write operation without an offset.
#[derive(Debug)]
pub struct Write {
    /// The state pointer associated with a writer (like stream, file, etc.).
    pub(crate) state_ptr: Ptr<State>,
    /// The buffer containing data to be written.
    pub(crate) buffer: Buffer,
    /// Pointer to store the result of the write operation.
    /// If success, the result will contain [`Buffer`] with the current `offset`
    /// (read [`Buffer`] or [`Buffer::offset`] description for more information).
    pub(crate) result_ptr: *mut Result<Buffer, Error>,
}

/// Contains all necessary information for a write all operation without an offset.
#[derive(Debug)]
pub struct WriteAll {
    /// The state pointer associated with a writer (like stream, file, etc.).
    pub(crate) state_ptr: Ptr<State>,
    /// The buffer containing data to be written.
    pub(crate) buffer: Buffer,
    /// Pointer to store the result of the write operation.
    /// If success, the result will contain [`Buffer`] with the current `offset`
    /// (read [`Buffer`] or [`Buffer::offset`] description for more information).
    pub(crate) result_ptr: *mut Result<Buffer, Error>,
}

/// Contains all necessary information for a positional read operation with an offset.
#[derive(Debug)]
pub struct PRead {
    /// The state pointer associated with a positional reader (like [`File`]).
    pub(crate) state_ptr: Ptr<State>,
    /// Offset from the beginning of [`File`].
    pub(crate) offset: usize,
    /// Pointer to store the result of the positional read operation.
    /// If success, the result will contain [`Buffer`].
    pub(crate) result_ptr: *mut Result<Buffer, Error>,
}

/// Contains all necessary information for a positional write operation with an offset.
#[derive(Debug)]
pub struct PWrite {
    /// The state pointer associated with a positional writer (like [`File`]).
    pub(crate) state_ptr: Ptr<State>,
    /// The buffer containing data to be written.
    pub(crate) buffer: Buffer,
    /// Offset from the beginning of [`File`].
    pub(crate) offset: usize,
    /// Pointer to store the result of the positional write operation.
    /// If success, the result will contain [`Buffer`] with the current `offset`
    /// (read [`Buffer`] or [`Buffer::offset`] description for more information).
    pub(crate) result_ptr: *mut Result<Buffer, Error>,
}

/// Contains all necessary information for a positional write all operation with an offset.
#[derive(Debug)]
pub struct PWriteAll {
    /// The state pointer associated with a positional  writer (like [`File`]).
    pub(crate) state_ptr: Ptr<State>,
    /// The buffer containing data to be written.
    pub(crate) buffer: Buffer,
    /// Offset from the beginning of [`File`].
    pub(crate) offset: usize,
    /// Pointer to store the result of the positional write operation.
    /// If success, the result will contain [`Buffer`] with the current `offset`
    /// (read [`Buffer`] or [`Buffer::offset`] description for more information).
    pub(crate) result_ptr: *mut Result<Buffer, Error>,
}

/// Contains all necessary information for a close operation.
#[derive(Debug)]
pub struct Close {
    /// The state pointer associated with a closable structure (like stream, file, etc.).
    pub(crate) state_ptr: Ptr<State>,
    /// Pointer to store the result of the close operation.
    /// If success, the result will contain `()`.
    pub(crate) result_ptr: *mut Result<(), Error>,
}

// TODO docs
pub struct Rename {
    pub(crate) old_path: Box<dyn AsRef<Path>>,
    pub(crate) new_path: Box<dyn AsRef<Path>>,
    pub(crate) result_ptr: *mut Result<(), Error>,
}

impl Debug for Rename {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Rename")
            .field("old_path", &self.old_path.as_ref().as_ref())
            .field("new_path", &self.new_path.as_ref().as_ref())
            .finish()
    }
}

// TODO docs
pub struct CreateDir {
    pub(crate) path: Box<dyn AsRef<Path>>,
    pub(crate) result_ptr: *mut Result<(), Error>,
}

impl_debug_for_as_ref_path!(CreateDir, "CreateDir");

// TODO docs
pub struct RemoveFile {
    pub(crate) path: Box<dyn AsRef<Path>>,
    pub(crate) result_ptr: *mut Result<(), Error>,
}

impl_debug_for_as_ref_path!(RemoveFile, "RemoveFile");

// TODO docs
pub struct RemoveDir {
    pub(crate) path: Box<dyn AsRef<Path>>,
    pub(crate) result_ptr: *mut Result<(), Error>,
}

impl_debug_for_as_ref_path!(RemoveDir, "RemoveDir");

/// The status of the coroutine yield. This is the one way to communicate with the scheduler.
/// It uses instead of await for async programming, and uses for creating new coroutines and for let the scheduler wake other coroutines up.
#[derive(Debug)]
pub enum YieldStatus {
    /// [`Yield`]
    ///
    /// If yielded, the coroutine let the scheduler wake other coroutines up.
    /// The current coroutine will be woken up by the scheduler after all other coroutines.
    Yield,
    
    // TODO Wait. It contains new task and replaces await

    /// [`Sleep`] takes the duration.
    ///
    /// # Arguments
    ///
    /// * [`Duration`] - The duration to sleep.
    ///
    /// If yielded, the coroutine will sleep for at least the duration.
    Sleep(Duration),

    /// [`NewTcpListener`]
    ///
    /// If yielded, a new [`TcpListener`] will be stored in the pointer.
    NewTcpListener(NewTcpListener),

    /// [`TcpConnect`]
    ///
    /// If yielded, a new [`TcpStream`] will be stored in the pointer.
    TcpConnect(TcpConnect),

    /// [`TcpAccept`]
    ///
    /// If yielded, the connection will be accepted and [`TcpStream`] will be stored in the result pointer.
    TcpAccept(TcpAccept),

    /// [`Recv`]
    ///
    /// If yielded, the connection assigned to this state will be read into the inner buffer.
    /// The read result will be stored in the result pointer.
    /// If successful, [`Buffer`] will be stored in the result pointer.
    /// If the length of [`Buffer`] is 0, the connection has been terminated by the other side.
    Recv(Recv),

    /// [`Send`]
    ///
    /// If yielded, a part of the buffer will be written (with a single syscall) to the connection assigned to this state.
    /// The write result will be stored in the result pointer. It will store the number of bytes written to the buffer if successful.
    Send(Send),

    /// [`SendAll`]
    ///
    /// If yielded, the buffer will be written whole (maybe with multiple syscalls) to the connection assigned to this state.
    /// The write result will be stored in the result pointer.
    SendAll(SendAll),
    
    /// [`OpenFile`]
    ///
    /// If yielded, the file will be opened and stored in the result pointer.
    OpenFile(OpenFile),

    /// [`Read`]
    ///
    /// If yielded, the reader assigned to this state will be read into the inner buffer.
    /// The read result will be stored in the result pointer.
    /// If successful, [`Buffer`] will be stored in the result pointer.
    /// If the length of [`Buffer`] is 0, the reader has been terminated by the other side.
    Read(Read),
    
    /// [`Write`]
    ///
    /// If yielded, a part of the buffer will be written (with a single syscall) to the writer assigned to this state.
    /// The write result will be stored in the result pointer. It will store the number of bytes written to the buffer if successful.
    Write(Write),
    
    /// [`WriteAll`]
    ///
    /// If yielded, the buffer will be written whole (maybe with multiple syscalls) to the writer assigned to this state.
    /// The write result will be stored in the result pointer.
    WriteAll(WriteAll),

    /// [`PRead`]
    ///
    /// If yielded, the reader assigned to this state will be read into the inner buffer.
    /// The read result will be stored in the result pointer.
    /// If successful, [`Buffer`] will be stored in the result pointer.
    /// If the length of [`Buffer`] is 0, the reader has been terminated by the other side.
    PRead(PRead),

    /// [`PWrite`]
    ///
    /// If yielded, a part of the buffer will be written (with a single syscall) to the writer assigned to this state.
    /// The write result will be stored in the result pointer. It will store the number of bytes written to the buffer if successful.
    PWrite(PWrite),

    /// [`PWriteAll`]
    ///
    /// If yielded, the buffer will be written whole (maybe with multiple syscalls) to the writer assigned to this state.
    /// The write result will be stored in the result pointer.
    PWriteAll(PWriteAll),

    /// [`Close`]
    /// 
    /// If yielded, the closable structure assigned to this state will be closed.
    /// The close result will be stored in the result pointer.
    Close(Close),
    
    // TODO docs
    Rename(Rename),
    
    // TODO docs
    CreateDir(CreateDir),
    
    // TODO docs
    RemoveFile(RemoveFile),

    // TODO docs
    RemoveDir(RemoveDir),
}

impl YieldStatus {
    /// Create a YieldStatus variant [`Yield`](YieldStatus::Yield).
    #[inline(always)]
    pub fn yield_now() -> Self {
        YieldStatus::Yield
    }

    /// Create a YieldStatus variant [`Sleep`](YieldStatus::Sleep).
    #[inline(always)]
    pub fn sleep(duration: Duration) -> Self {
        YieldStatus::Sleep(duration)
    }

    /// Create a YieldStatus variant [`NewTcpListener`](YieldStatus::NewTcpListener).
    #[inline(always)]
    pub fn new_tcp_listener(address: SocketAddr, listener_ptr: *mut TcpListener) -> Self {
        YieldStatus::NewTcpListener(NewTcpListener { address, listener_ptr })
    }

    /// Create a YieldStatus variant [`TcpConnect`](YieldStatus::TcpConnect).
    #[inline(always)]
    pub fn tcp_connect(address: SocketAddr, result_ptr: *mut Result<TcpStream, Error>) -> Self {
        YieldStatus::TcpConnect(TcpConnect { address, stream_ptr: result_ptr })
    }

    /// Create a YieldStatus variant [`TcpAccept`](YieldStatus::TcpAccept).
    #[inline(always)]
    pub fn tcp_accept(state_ptr: Ptr<State>, result_ptr: *mut Result<TcpStream, Error>) -> Self {
        YieldStatus::TcpAccept(TcpAccept { state_ptr, result_ptr })
    }

    /// Create a YieldStatus variant [`TcpRead`](YieldStatus::Recv).
    #[inline(always)]
    pub fn recv(state_ref: Ptr<State>, result_ptr: *mut Result<Buffer, Error>) -> Self {
        YieldStatus::Recv(Recv { state_ptr: state_ref, result_ptr })
    }

    /// Create a YieldStatus variant [`TcpWrite`](YieldStatus::Send).
    #[inline(always)]
    pub fn send(state_ref: Ptr<State>, buffer: Buffer, result_ptr: *mut Result<Buffer, Error>) -> Self {
        YieldStatus::Send(Send { state_ptr: state_ref, buffer, result_ptr })
    }

    /// Create a YieldStatus variant [`TcpWriteAll`](YieldStatus::SendAll).
    #[inline(always)]
    pub fn send_all(state_ref: Ptr<State>, buffer: Buffer, result_ptr: *mut Result<Buffer, Error>) -> Self {
        YieldStatus::SendAll(SendAll { state_ptr: state_ref, buffer, result_ptr })
    }
    
    /// Create a YieldStatus variant [`OpenFile`](YieldStatus::OpenFile).
    #[inline(always)]
    pub fn open_file(path: Box<dyn AsRef<Path>>, options: OpenOptions, result_ptr: *mut Result<File, Error>) -> Self {
        YieldStatus::OpenFile(OpenFile { path, options, file_ptr: result_ptr })
    }
    
    /// Create a YieldStatus variant [`Read`](YieldStatus::Read).
    #[inline(always)]
    pub fn read(state_ref: Ptr<State>, result_ptr: *mut Result<Buffer, Error>) -> Self {
        YieldStatus::Read(Read { state_ptr: state_ref, result_ptr })
    }
    
    /// Create a YieldStatus variant [`Write`](YieldStatus::Write).
    #[inline(always)]
    pub fn write(state_ref: Ptr<State>, buffer: Buffer, result_ptr: *mut Result<Buffer, Error>) -> Self {
        YieldStatus::Write(Write { state_ptr: state_ref, buffer, result_ptr })
    }
    
    /// Create a YieldStatus variant [`WriteAll`](YieldStatus::WriteAll).
    #[inline(always)]
    pub fn write_all(state_ref: Ptr<State>, buffer: Buffer, result_ptr: *mut Result<Buffer, Error>) -> Self {
        YieldStatus::WriteAll(WriteAll { state_ptr: state_ref, buffer, result_ptr })
    }
    
    /// Create a YieldStatus variant [`PRead`](YieldStatus::PRead).
    #[inline(always)]
    pub fn pread(state_ref: Ptr<State>, offset: usize, result_ptr: *mut Result<Buffer, Error>) -> Self {
        YieldStatus::PRead(PRead { state_ptr: state_ref, offset, result_ptr })
    }
    
    /// Create a YieldStatus variant [`PWrite`](YieldStatus::PWrite).
    #[inline(always)]
    pub fn pwrite(state_ref: Ptr<State>, offset: usize, buffer: Buffer, result_ptr: *mut Result<Buffer, Error>) -> Self {
        YieldStatus::PWrite(PWrite { state_ptr: state_ref, offset, buffer, result_ptr })
    }
    
    /// Create a YieldStatus variant [`PWriteAll`](YieldStatus::PWriteAll).
    #[inline(always)]
    pub fn pwrite_all(state_ref: Ptr<State>, offset: usize, buffer: Buffer, result_ptr: *mut Result<Buffer, Error>) -> Self {
        YieldStatus::PWriteAll(PWriteAll { state_ptr: state_ref, offset, buffer, result_ptr })
    }

    /// Create a YieldStatus variant [`TcpClose`](YieldStatus::Close).
    #[inline(always)]
    pub fn close(state_ref: Ptr<State>, result_ptr: *mut Result<(), Error>) -> Self {
        YieldStatus::Close(Close { state_ptr: state_ref, result_ptr })
    }
    
    #[inline(always)]
    pub fn rename(old_path: Box<dyn AsRef<Path>>, new_path: Box<dyn AsRef<Path>>, result_ptr: *mut Result<(), Error>) -> Self {
        YieldStatus::Rename(Rename { old_path, new_path, result_ptr })
    }
    
    #[inline(always)]
    pub fn create_dir(path: Box<dyn AsRef<Path>>, result_ptr: *mut Result<(), Error>) -> Self {
        YieldStatus::CreateDir(CreateDir { path, result_ptr })
    }
    
    // TODO docs
    #[inline(always)]
    pub fn remove_file(path: Box<dyn AsRef<Path>>, result_ptr: *mut Result<(), Error>) -> Self {
        YieldStatus::RemoveFile(RemoveFile { path, result_ptr })
    }
    
    // TODO docs
    #[inline(always)]
    pub fn remove_dir(path: Box<dyn AsRef<Path>>, result_ptr: *mut Result<(), Error>) -> Self {
        YieldStatus::RemoveDir(RemoveDir { path, result_ptr })
    }
}