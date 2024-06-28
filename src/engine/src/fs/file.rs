use std::io::Error;
use std::path::Path;
use crate::buf::Buffer;
use crate::coroutine::YieldStatus;
use crate::fs::OpenOptions;
// TODO docs
use crate::io::{AsyncPRead, AsyncPWrite, AsyncRead, AsyncWrite, State};
use crate::utils::Ptr;

// TODO docs
pub struct File {
    state_ptr: Ptr<State>
}

impl File {
    /// Creates a new [`File`] struct from an existing fd.
    pub fn from_state_ptr(state_ptr: Ptr<State>) -> Self {
        Self {
            state_ptr
        }
    }
    
    pub fn open(path: Box<dyn AsRef<Path>>, options: OpenOptions, res: *mut Result<File, Error>) -> YieldStatus {
        YieldStatus::open_file(path, options, res)
    }

    #[inline(always)]
    pub fn remove(path: Box<dyn AsRef<Path>>, res: *mut Result<(), Error>) -> YieldStatus {
        YieldStatus::remove_file(path, res)
    }
}

impl AsyncRead for File {
    fn read(&mut self, res: *mut Result<Buffer, Error>) -> YieldStatus {
        YieldStatus::read(self.state_ptr, res)
    }
}

impl AsyncPRead for File {
    fn pread(&mut self, offset: usize, res: *mut Result<Buffer, Error>) -> YieldStatus {
        YieldStatus::pread(self.state_ptr, offset, res)
    }
}

impl AsyncWrite for File {
    fn write(&mut self, buffer: Buffer, res: *mut Result<Buffer, Error>) -> YieldStatus {
        YieldStatus::write(self.state_ptr, buffer, res)
    }

    fn write_all(&mut self, data: Buffer, res: *mut Result<Buffer, Error>) -> YieldStatus {
        YieldStatus::write_all(self.state_ptr, data, res)
    }
}

impl AsyncPWrite for File {
    fn pwrite(&mut self, buffer: Buffer, offset: usize, res: *mut Result<Buffer, Error>) -> YieldStatus {
        YieldStatus::pwrite(self.state_ptr, offset, buffer, res)
    }
    
    fn pwrite_all(&mut self, data: Buffer, offset: usize, res: *mut Result<Buffer, Error>) -> YieldStatus {
        YieldStatus::pwrite_all(self.state_ptr, offset, data, res)
    }
}

impl Drop for File {
    fn drop(&mut self) {
        // TODO
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proc::test_local;

    #[test_local(crate="crate")]
    fn new() {

    }
}